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
use std::borrow::Cow;
use std::rc::Rc;

use anyhow::anyhow;
use gettextrs::gettext;
use glib::prelude::*;
use glib::translate::IntoGlib;
use log::{debug, warn};
use rdw_vnc::gvnc::ConnectionCredential;
use secure_string::SecureString;

use crate::adapter::types::{Adapter, AdapterDisplay};
use crate::connection::ConnectionError;

pub struct VncAdapter {
    host: String,
    port: u32,
    user: String,
    password: SecureString,
}

impl VncAdapter {
    pub const TAG: &'static str = "vnc";

    pub fn new(host: String, port: u32, user: String, password: SecureString) -> Self {
        Self {
            host,
            port,
            user,
            password,
        }
    }

    pub fn label() -> Cow<'static, str> {
        gettext("VNC").into()
    }
}

impl Adapter for VncAdapter {
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
    ) -> AdapterDisplay {
        let user = self.user.clone();

        let vnc = rdw_vnc::Display::new();
        vnc.connection()
            .open_host(&self.host, &format!("{}", self.port))
            .unwrap();

        vnc.connection().connect_vnc_error(glib::clone!(
            #[strong]
            on_disconnected,
            move |_conn, err| {
                warn!("VNC connect error: {:?}", &err);
                let err_msg = err.to_string();
                on_disconnected(Err(ConnectionError::General(
                    Some(err_msg),
                    anyhow!("{}", &err),
                )));
            }
        ));

        vnc.connection().connect_vnc_auth_failure(glib::clone!(
            #[strong]
            on_disconnected,
            move |_conn, err| {
                warn!("VNC auth failure: {:?}", &err);
                let err_msg = err.to_string();
                on_disconnected(Err(ConnectionError::AuthFailed(
                    Some(err_msg),
                    anyhow!("{}", &err),
                )));
            }
        ));

        vnc.connection().connect_vnc_disconnected(glib::clone!(
            #[strong]
            on_disconnected,
            move |_conn| {
                debug!("VNC connection disconnected");
                on_disconnected(Ok(()));
            }
        ));

        vnc.connection().connect_vnc_connected(glib::clone!(
            #[strong]
            on_connected,
            move |_conn| {
                debug!("VNC connection established");
                on_connected();
            }
        ));

        vnc.connection()
            .connect_vnc_auth_credential(move |conn, va| {
                debug!("VNC connection authenticating");
                let creds: Vec<_> = va
                    .iter()
                    .map(|v| v.get::<ConnectionCredential>().unwrap())
                    .collect();
                if creds.contains(&ConnectionCredential::Username) {
                    conn.set_credential(ConnectionCredential::Username.into_glib(), &user)
                        .unwrap();
                }
                if creds.contains(&ConnectionCredential::Clientname) {
                    conn.set_credential(
                        ConnectionCredential::Clientname.into_glib(),
                        "field-monitor",
                    )
                    .unwrap();
                }
                if creds.contains(&ConnectionCredential::Password) {
                    conn.set_credential(
                        ConnectionCredential::Password.into_glib(),
                        self.password.unsecure(),
                    )
                    .unwrap();
                }
            });

        AdapterDisplay::Rdw(vnc.upcast())
    }
}
