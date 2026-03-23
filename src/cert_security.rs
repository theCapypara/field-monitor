/* Copyright 2024-2026 Marco Köpcke
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use crate::util::config_dir;
use anyhow::anyhow;
use glib::{Checksum, ChecksumType};
use libfieldmonitor::cert_security::{Encode, X509Certificate};
use log::{error, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io;
use std::io::BufRead;
use std::io::Write;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};

pub const TRUST_FILENAME: &str = "trusted_certs";

/// Field Monitor's certificate trust store. Can be used to store trust for self-signed
/// or otherwise untrusted certificates.
pub struct FieldMonitorTrustStore {
    pub path: PathBuf,
    pub store: RefCell<HashMap<String, Vec<Box<[u8]>>>>,
}

impl FieldMonitorTrustStore {
    pub fn load_default() -> Self {
        let path = config_dir().join(TRUST_FILENAME);
        if !path.exists() {
            Self {
                path,
                store: Default::default(),
            }
        } else {
            match Self::parse(&path) {
                Ok(store) => Self {
                    path,
                    store: RefCell::new(store),
                },
                Err(err) => {
                    error!(
                        "Failed to load app's certificate trust store ({}): {}. Continuing with empty store.",
                        path.display(),
                        err
                    );
                    Self {
                        path,
                        store: Default::default(),
                    }
                }
            }
        }
    }

    fn parse(path: &Path) -> io::Result<HashMap<String, Vec<Box<[u8]>>>> {
        let mut out: HashMap<String, Vec<Box<[u8]>>> = HashMap::new();
        let file = OpenOptions::new().read(true).open(path)?;
        let reader = io::BufReader::new(file).lines();

        for line in reader {
            let line = line?;
            let mut tab_iter = line.split('\t');
            let Some(host) = tab_iter.next() else {
                warn!(
                    "Invalid entry in certificate trust store ({}).",
                    path.display()
                );
                continue;
            };
            let Some(fingerprint_str) = tab_iter.next() else {
                warn!(
                    "Invalid entry in certificate trust store ({}).",
                    path.display()
                );
                continue;
            };
            let Ok(fingerprint_digest) = Self::digest_to_bytes(fingerprint_str) else {
                warn!(
                    "Invalid entry in certificate trust store ({}): invalid hash",
                    path.display()
                );
                continue;
            };

            out.entry(host.to_string())
                .and_modify(|entry| entry.push(fingerprint_digest.clone()))
                .or_insert_with(|| vec![fingerprint_digest]);
        }

        Ok(out)
    }

    pub fn verify(&self, cert: &X509Certificate, for_host: &str) -> anyhow::Result<bool> {
        let checksum = Self::checksum_for_cert(cert)?;
        Ok(self
            .store
            .borrow()
            .get(for_host)
            .map(|trusted| trusted.contains(&checksum.digest().into_boxed_slice()))
            .unwrap_or_default())
    }

    pub fn trust(&self, cert: &X509Certificate, for_host: &str) -> anyhow::Result<()> {
        let checksum = Self::checksum_for_cert(cert)?;
        let digest = checksum.digest().into_boxed_slice();
        let fingerprint = format_bytes_as_hex_string(&digest);
        self.store
            .borrow_mut()
            .entry(for_host.to_string())
            .and_modify(|entry| entry.push(digest.clone()))
            .or_insert_with(|| vec![digest]);
        self.append_to_file(for_host, &fingerprint)?;
        Ok(())
    }

    fn append_to_file(&self, host: &str, fingerprint: &str) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{host}\t{fingerprint}")
    }

    pub fn checksum_for_cert(cert: &X509Certificate) -> anyhow::Result<Checksum> {
        let mut checksum = Checksum::new(ChecksumType::Sha256)
            .ok_or_else(|| anyhow!("could not create SHA256"))?;
        checksum.update(&cert.to_der()?);
        Ok(checksum)
    }

    pub fn make_fingerprint_digest(cert: &X509Certificate) -> anyhow::Result<String> {
        let checksum = Self::checksum_for_cert(cert)?;
        let fingerprint = checksum.digest();
        Ok(format_bytes_as_hex_string(fingerprint))
    }

    fn digest_to_bytes(digest: &str) -> Result<Box<[u8]>, ParseIntError> {
        let digest = digest
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();
        (0..digest.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&digest[i..i + 2], 16))
            .collect()
    }
}

pub fn format_bytes_as_hex_string(bytes: impl AsRef<[u8]>) -> String {
    let mut out_str = String::new();
    for byte in bytes.as_ref() {
        out_str += &*format!("{:02X}:", byte);
    }
    out_str.pop();
    out_str
}
