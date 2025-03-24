/* Copyright 2024-2025 Marco Köpcke
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
use std::env;
use std::process::exit;

use crate::dbus_client::{VtePtyProcMonProxy, make_dbus_client};
use crate::{DBUS_KEY_ENV_VAR, debug};

pub struct PtyClient {
    extra_args: Vec<String>,
    dbus_client: VtePtyProcMonProxy<'static>,
}

impl PtyClient {
    async fn new_from_env() -> zbus::Result<Self> {
        let extra_args_key = env::var(DBUS_KEY_ENV_VAR).unwrap_or_default();
        let name = env::args().nth(1).unwrap_or_default();
        let dbus_client = make_dbus_client(&name).await?;
        let extra_args = dbus_client.extra_arguments(&extra_args_key).await?;
        let slf = Self {
            extra_args,
            dbus_client,
        };

        debug!(&slf, "setup pty client");

        Ok(slf)
    }

    pub async fn set_result<S1, S2>(&self, result: Result<S1, S2>) -> zbus::Result<()>
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        match result {
            Ok(s) => self.dbus_client.set_result(false, s.as_ref()).await,
            Err(s) => self.dbus_client.set_result(true, s.as_ref()).await,
        }
    }

    pub fn args(&self) -> &[String] {
        &self.extra_args
    }

    pub async fn log_debug(&self, msg: &str) {
        self.dbus_client.log_debug(msg).await.ok();
    }

    pub async fn log_warn(&self, msg: &str) {
        self.dbus_client.log_warn(msg).await.ok();
    }

    pub async fn log_error(&self, msg: &str) {
        self.dbus_client.log_error(msg).await.ok();
    }
}

pub async fn setup_driver() -> PtyClient {
    match PtyClient::new_from_env().await {
        Ok(dri) => dri,
        Err(err) => {
            eprintln!("Failed to setup Pty driver: {err}");
            exit(44);
        }
    }
}
