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
use glib::translate::{FromGlib, IntoGlib};
use log::{debug, warn};
use rdw_vnc::gvnc;
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

        vnc.connection()
            .connect_vnc_auth_credential(move |conn, va| {
                debug!("VNC connection authenticating");
                let creds: Vec<_> = va
                    .iter()
                    .map(|v| v.get::<gvnc::ConnectionCredential>().unwrap())
                    .collect();
                dbg!(&creds);
                if creds.contains(&gvnc::ConnectionCredential::Username) {
                    dbg!("username", &user);
                    conn.set_credential(gvnc::ConnectionCredential::Username.into_glib(), &user)
                        .unwrap();
                }
                if creds.contains(&gvnc::ConnectionCredential::Clientname) {
                    dbg!("clientname", "field-monitor");
                    conn.set_credential(
                        gvnc::ConnectionCredential::Clientname.into_glib(),
                        "field-monitor",
                    )
                    .unwrap();
                }
                if creds.contains(&gvnc::ConnectionCredential::Password) {
                    dbg!("password", self.password.unsecure());
                    conn.set_credential(
                        gvnc::ConnectionCredential::Password.into_glib(),
                        self.password.unsecure(),
                    )
                    .unwrap();
                }
            });

        vnc.connection()
            .connect_vnc_auth_choose_subtype(|conn, typ, va| {
                // SAFETY: We trust that gvnc gives us a valid type.
                match unsafe { gvnc::ConnectionAuth::from_glib(typ as i32) } {
                    gvnc::ConnectionAuth::Vencrypt => {
                        let prefer_subauth = [
                            gvnc::ConnectionAuthVencrypt::Tlsvnc,
                            gvnc::ConnectionAuthVencrypt::Tlssasl,
                            gvnc::ConnectionAuthVencrypt::Tlsplain,
                            gvnc::ConnectionAuthVencrypt::Tlsnone,
                            gvnc::ConnectionAuthVencrypt::X509sasl,
                            gvnc::ConnectionAuthVencrypt::X509vnc,
                            gvnc::ConnectionAuthVencrypt::X509plain,
                            gvnc::ConnectionAuthVencrypt::X509none,
                            gvnc::ConnectionAuthVencrypt::Plain,
                        ];
                        for &auth in &prefer_subauth {
                            for a in va.iter() {
                                if a.get::<gvnc::ConnectionAuthVencrypt>().unwrap() == auth {
                                    if let Err(e) =
                                        conn.set_auth_subtype(auth.into_glib().try_into().unwrap())
                                    {
                                        warn!("Failed to set auth subtype: {}", e);
                                        conn.shutdown();
                                    }
                                    return;
                                }
                            }
                        }

                        warn!("No preferred auth subtype found");
                        conn.shutdown();
                    }
                    typ => {
                        warn!("unknown how to set vnc subtype for type {typ:?}");
                        conn.shutdown();
                    }
                }
            });

        vnc.connection()
            .open_host(&host, &format!("{}", port))
            .unwrap();

        AdapterDisplay::Rdw(vnc.upcast())
    }
}
