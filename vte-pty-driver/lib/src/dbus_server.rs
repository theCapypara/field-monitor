/* Copyright 2024 Marco Köpcke
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
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use base64::prelude::*;
use parking_lot::Mutex;
use zbus::{connection, interface, Connection};

use crate::DBUS_PATH;

pub struct VtePtyProcMon {
    name: Arc<String>,
    result: Arc<Mutex<Option<Result<String, String>>>>,
    extra_args: Vec<String>,
    fm_key: String,
}

pub struct RunningVtePtyProcMon {
    conn: Connection,
    name: Arc<String>,
    result: Arc<Mutex<Option<Result<String, String>>>>,
}

#[interface(name = "de.capypara.FieldMonitor.VtePtyProcMon1")]
impl VtePtyProcMon {
    fn extra_arguments(&mut self, fm_key: &str) -> zbus::fdo::Result<&[String]> {
        if self.fm_key == fm_key {
            Ok(&self.extra_args)
        } else {
            Err(zbus::fdo::Error::AccessDenied(String::new()))
        }
    }

    fn set_result(&mut self, is_err: bool, msg: &str) {
        log::debug!("{} result: err?:{} msg:{}", self.name, is_err, msg);
        self.result.lock().replace(if is_err {
            Err(msg.to_string())
        } else {
            Ok(msg.to_string())
        });
    }

    fn log_debug(&mut self, msg: &str) {
        log::debug!("{} client: {}", self.name, msg);
    }

    fn log_error(&mut self, msg: &str) {
        log::error!("{} client: {}", self.name, msg);
    }

    fn log_warn(&mut self, msg: &str) {
        log::warn!("{} client: {}", self.name, msg);
    }
}

impl VtePtyProcMon {
    pub async fn server(
        connection_id: &str,
        server_id: &str,
        adapter_id: &str,
        extra_args: &[String],
        fm_key: &str,
    ) -> Result<RunningVtePtyProcMon, zbus::Error> {
        let connection_id_ne = format!("con-{}", hash(connection_id));
        let server_id_ne = format!("srv-{}", hash(server_id));
        let adapter_id_ne = format!("adp-{}", hash(adapter_id));
        let name = [
            "de",
            "capypara",
            "FieldMonitor",
            "VtePtyProcMon",
            &connection_id_ne,
            &server_id_ne,
            &adapter_id_ne,
        ]
        .join(".");
        log::debug!("starting VtePtyProcMon with name {name}");
        let name = Arc::new(name);
        let result: Arc<Mutex<_>> = Default::default();
        let slf = VtePtyProcMon {
            name: name.clone(),
            result: result.clone(),
            extra_args: extra_args.to_vec(),
            fm_key: fm_key.to_string(),
        };
        let conn = connection::Builder::session()?
            .name(name.to_string())?
            .serve_at(DBUS_PATH, slf)?
            .build()
            .await?;

        Ok(RunningVtePtyProcMon { conn, name, result })
    }
}

impl RunningVtePtyProcMon {
    pub async fn close(self) -> zbus::Result<()> {
        self.conn.close().await
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn result(&self) -> &Mutex<Option<Result<String, String>>> {
        &self.result
    }
}

fn hash(v: &str) -> String {
    let mut hasher = DefaultHasher::new();
    v.hash(&mut hasher);
    BASE64_URL_SAFE_NO_PAD.encode(hasher.finish().to_le_bytes())
}
