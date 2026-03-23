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

//! TLS verification module.
//! Handles trust of TLS server certificates and provides hooks for interactively
//! handling certificates that aren't trusted.

use crate::connection::{ConnectionError, ConnectionResult};
use anyhow::anyhow;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use log::warn;
use rustls::client::danger::ServerCertVerifier;
use rustls::pki_types;
use rustls::pki_types::{ServerName, UnixTime};
use rustls_platform_verifier::Verifier;
use std::sync::{Arc, LazyLock};
use x509_cert::der::oid::db::rfc4519::COMMON_NAME;
pub use x509_cert::der::{Decode, Encode};
use x509_cert::name::Name;

pub use x509_cert::Certificate as X509Certificate;

static VERIFIER: LazyLock<Result<Verifier, rustls::Error>> =
    LazyLock::new(|| Verifier::new(Arc::new(rustls_openssl::default_provider())));

/// Adapter for TLS verification.
#[derive(Debug, Clone)]
pub struct VerifyTls {
    certs: VerifiableCertChain,
    host: String,
    verify_cert_subject: Option<Name>,
    mode: VerifyTlsMode,
    is_ca: bool,
}

impl VerifyTls {
    pub(crate) fn verify_async(
        certs: VerifiableCertChain,
        host: impl AsRef<str>,
        verify_cert_subject: Option<Name>,
        is_ca: bool,
    ) -> Self {
        Self {
            certs,
            host: host.as_ref().to_string(),
            verify_cert_subject,
            mode: VerifyTlsMode::Async,
            is_ca,
        }
    }
    pub(crate) fn verify_sync(
        certs: VerifiableCertChain,
        host: impl AsRef<str>,
        verify_cert_subject: Option<Name>,
        is_ca: bool,
    ) -> Self {
        Self {
            certs,
            host: host.as_ref().to_string(),
            verify_cert_subject,
            mode: VerifyTlsMode::Sync,
            is_ca,
        }
    }

    pub fn mode(&self) -> VerifyTlsMode {
        self.mode
    }

    pub fn is_ca(&self) -> bool {
        self.is_ca
    }

    /// Verify the certificate against the system CA.
    pub fn verify_system(&self) -> bool {
        let Ok(host) = ServerName::try_from(&*self.host) else {
            warn!("tls verify: invalid subject dns name");
            return false;
        };

        if let Some(expected) = &self.verify_cert_subject {
            let actual = &self.certs.main.0.tbs_certificate.subject;
            if actual != expected {
                warn!(
                    "tls verify: subject lines did not match. Expected: '{}', Actual: '{}'",
                    expected, actual
                );
                return false;
            }
        }

        VERIFIER
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|verifier| {
                verifier.verify_server_cert(
                    self.certs.main_rustls(),
                    &self
                        .certs
                        .intermediates_rustls()
                        .cloned()
                        .collect::<Box<[_]>>(),
                    &host,
                    &[],
                    UnixTime::now(),
                )
            })
            .inspect_err(|err| warn!("failed cert verification (for {}): {}", &self.host, err))
            .is_ok()
    }

    pub fn into_verification_info(self) -> (X509Certificate, String) {
        (self.certs.main.0, self.host)
    }

    pub fn error() -> ConnectionError {
        ConnectionError::General(
            Some(gettext("Server certificate verification failed")),
            anyhow!("Server certificate verification failed"),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VerifyTlsMode {
    Async,
    Sync,
}

pub enum VerifyTlsResponse {
    Async(LocalBoxFuture<'static, bool>),
    Sync(bool),
}

impl VerifyTlsResponse {
    pub fn make_static(mode: VerifyTlsMode, value: bool) -> VerifyTlsResponse {
        match mode {
            VerifyTlsMode::Async => VerifyTlsResponse::Async(Box::pin(async move { value })),
            VerifyTlsMode::Sync => VerifyTlsResponse::Sync(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VerifiableCertChain {
    main: Cert,
    intermediates: Vec<Cert>,
}

impl VerifiableCertChain {
    pub(crate) fn from_cert(cert: impl AsRef<[u8]>) -> ConnectionResult<Self> {
        let cert = match X509Certificate::from_der(cert.as_ref()) {
            Ok(cert) => cert,
            Err(err) => {
                return Err(ConnectionError::General(None, err.into()));
            }
        };
        Ok(Self {
            main: Cert::from(cert),
            intermediates: vec![],
        })
    }

    pub(crate) fn from_pem_chain(pems: impl AsRef<[u8]>) -> ConnectionResult<Self> {
        // TODO: It's probably not correct to assume the first cert is the root CA cert and others
        //       the intermediates? We should probably actually build the chain?
        let mut other_certs = match X509Certificate::load_pem_chain(pems.as_ref()) {
            Ok(certs) => certs,
            Err(err) => {
                return Err(ConnectionError::General(None, err.into()));
            }
        };
        let main_cert = if !other_certs.is_empty() {
            other_certs.remove(0)
        } else {
            return Err(ConnectionError::General(
                None,
                anyhow!("No certificate provided"),
            ));
        };

        Ok(Self {
            main: Cert::from(main_cert),
            intermediates: other_certs.into_iter().map(Cert::from).collect(),
        })
    }

    pub(crate) fn main_rustls(&'_ self) -> &'_ pki_types::CertificateDer<'_> {
        &self.main.1
    }
    pub(crate) fn intermediates_rustls(
        &'_ self,
    ) -> impl Iterator<Item = &'_ pki_types::CertificateDer<'_>> {
        self.intermediates.iter().map(|c| &c.1)
    }
}

/// A certificate stored as rustls and x905 crate format
#[derive(Debug, Clone)]
pub struct Cert(X509Certificate, pki_types::CertificateDer<'static>);

impl From<X509Certificate> for Cert {
    fn from(value: X509Certificate) -> Self {
        let der = value.to_der().unwrap();
        Self(value, pki_types::CertificateDer::from(der))
    }
}

pub fn extract_common_name(name: &Name) -> Option<String> {
    for entry in &name.0 {
        for attr in entry.0.iter() {
            if attr.oid == COMMON_NAME
                && let Ok(cn) = str::from_utf8(attr.value.value())
            {
                return Some(cn.to_owned());
            }
        }
    }
    None
}
