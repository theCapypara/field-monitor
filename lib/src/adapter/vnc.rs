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
use std::cell::RefCell;
use std::rc::Rc;

use anyhow::anyhow;
use gettextrs::gettext;
use glib::prelude::*;
use glib::translate::IntoGlib;
use log::{debug, warn};
use rdw_vnc::gvnc;
use secure_string::SecureString;

use crate::adapter::types::{Adapter, AdapterDisplay, AdapterDisplayWidget};
use crate::connection::ConnectionError;

pub struct VncAdapter {
    host: String,
    port: u32,
    user: String,
    password: SecureString,
    ca: Option<String>,
}

impl VncAdapter {
    pub const TAG: &'static str = "vnc";

    pub fn new(host: String, port: u32, user: String, password: SecureString) -> Self {
        Self {
            host,
            port,
            user,
            password,
            ca: None,
        }
    }

    pub fn new_with_ca(
        host: String,
        port: u32,
        user: String,
        password: SecureString,
        ca: String,
    ) -> Self {
        Self {
            host,
            port,
            user,
            password,
            ca: Some(ca),
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
    ) -> Box<dyn AdapterDisplay> {
        debug!("creating vnc adapter");
        let error_container: Rc<RefCell<Option<ConnectionError>>> = Rc::new(RefCell::new(None));
        let host = self.host.clone();
        let user = self.user.clone();
        let port = self.port;

        let vnc = rdw_vnc::Display::new();

        let error_container2 = error_container.clone();
        vnc.connection().connect_vnc_error(move |_conn, err| {
            warn!("VNC connect error: {:?}", &err);
            let err_msg = err.to_string();

            if error_container2.borrow().is_none() {
                error_container2.replace(Some(ConnectionError::General(
                    Some(err_msg),
                    anyhow!("{}", &err),
                )));
            }
        });

        let error_container3 = error_container.clone();
        vnc.connection()
            .connect_vnc_auth_failure(move |_conn, err| {
                warn!("VNC auth failure: {:?}", &err);
                let err_msg = err.to_string();
                error_container3.replace(Some(ConnectionError::AuthFailed(
                    Some(err_msg),
                    anyhow!("{}", &err),
                )));
            });

        vnc.connection().connect_vnc_disconnected(move |_conn| {
            debug!("VNC connection disconnected");
            match error_container.borrow_mut().take() {
                None => on_disconnected(Ok(())),
                Some(err) => on_disconnected(Err(err)),
            }
        });

        vnc.connection().connect_vnc_connected(move |_conn| {
            debug!("VNC connection established");
            on_connected();
        });

        let ca = Rc::new(self.ca.clone());

        vnc.connection().connect_vnc_auth_credential(glib::clone!(
            #[strong]
            ca,
            move |conn, va| {
                debug!("VNC connection authenticating");
                let creds: Vec<_> = va
                    .iter()
                    .map(|v| v.get::<gvnc::ConnectionCredential>().unwrap())
                    .collect();
                if creds.contains(&gvnc::ConnectionCredential::Username) {
                    conn.set_credential(gvnc::ConnectionCredential::Username.into_glib(), &user)
                        .unwrap();
                }
                if creds.contains(&gvnc::ConnectionCredential::Clientname) {
                    conn.set_credential(
                        gvnc::ConnectionCredential::Clientname.into_glib(),
                        "field-monitor",
                    )
                    .unwrap();
                }
                if creds.contains(&gvnc::ConnectionCredential::Password) {
                    conn.set_credential(
                        gvnc::ConnectionCredential::Password.into_glib(),
                        self.password.unsecure(),
                    )
                    .unwrap();
                }

                // TODO: gtk-vnc with this option is not released as stable yet, and we don't
                //       want to bother updating the Rust bindings to the unstable release,
                //       so we use this instead.
                //       In the future this will be gvnc::ConnectionCredential::CaCertData probably.
                const VNC_CONNECTION_CREDENTIAL_CA_CERT_DATA: i32 = 3;
                if let Some(ca) = &*ca {
                    if creds.contains(&gvnc::ConnectionCredential::__Unknown(
                        VNC_CONNECTION_CREDENTIAL_CA_CERT_DATA,
                    )) {
                        debug!("providing CA cert");
                        conn.set_credential(VNC_CONNECTION_CREDENTIAL_CA_CERT_DATA, ca)
                            .unwrap();
                    }
                }
            }
        ));

        vnc.connection()
            .open_host(&host, &format!("{}", port))
            .unwrap();

        Box::new(VncAdapterDisplay(vnc))
    }
}

pub struct VncAdapterDisplay(rdw_vnc::Display);

impl AdapterDisplay for VncAdapterDisplay {
    fn widget(&self) -> AdapterDisplayWidget {
        AdapterDisplayWidget::Rdw(self.0.clone().upcast())
    }

    fn close(&self) {
        self.0.connection().shutdown()
    }
}

impl Drop for VncAdapterDisplay {
    fn drop(&mut self) {
        self.close()
    }
}
