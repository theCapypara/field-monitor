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
use crate::adapter::types::{Adapter, AdapterDisplay, AdapterDisplayWidget};
use crate::cert_security::{VerifiableCertChain, VerifyTls, VerifyTlsResponse};
use crate::connection::{ConnectionError, ConnectionResult};
use anyhow::anyhow;
use gettextrs::gettext;
use glib::prelude::*;
use glib::translate::IntoGlib;
use log::{debug, warn};
use rdw_vnc::gvnc;
use secure_string::SecureString;
use std::borrow::Cow;
use std::cell::RefCell;
use std::mem;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::rc::Rc;

trait MakeSession {
    fn connect(self, connection: &gvnc::Connection) -> Result<(), glib::BoolError>;
}

struct VncNetworkConfig {
    host: String,
    port: u32,
    ca: Option<String>,
}

impl MakeSession for VncNetworkConfig {
    fn connect(self, connection: &gvnc::Connection) -> Result<(), glib::BoolError> {
        connection.open_host(&self.host, &format!("{}", self.port))
    }
}

struct VncSocketConfig {
    stream: UnixStream,
}

impl MakeSession for VncSocketConfig {
    fn connect(self, connection: &gvnc::Connection) -> Result<(), glib::BoolError> {
        if let Err(err) = connection.open_fd(self.stream.as_raw_fd()) {
            Err(err)
        } else {
            mem::forget(self.stream);
            Ok(())
        }
    }
}

enum VncAdapterMode {
    /// The connection is made over the network.
    Network(VncNetworkConfig),
    /// The connection is made using a socket
    Socket(VncSocketConfig),
}

impl MakeSession for VncAdapterMode {
    fn connect(self, connection: &gvnc::Connection) -> Result<(), glib::BoolError> {
        match self {
            VncAdapterMode::Network(c) => c.connect(connection),
            VncAdapterMode::Socket(c) => c.connect(connection),
        }
    }
}

pub struct VncAdapter {
    cfg: VncAdapterMode,
    user: Option<String>,
    password: Option<SecureString>,
}

impl VncAdapter {
    pub const TAG: &'static str = "vnc";

    pub fn new(host: String, port: u32, user: String, password: SecureString) -> Self {
        Self {
            cfg: VncAdapterMode::Network(VncNetworkConfig {
                host,
                port,
                ca: None,
            }),
            user: Some(user),
            password: Some(password),
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
            cfg: VncAdapterMode::Network(VncNetworkConfig {
                host,
                port,
                ca: Some(ca),
            }),
            user: Some(user),
            password: Some(password),
        }
    }

    pub fn new_from_socket(
        stream: UnixStream,
        user: Option<String>,
        password: Option<SecureString>,
    ) -> Self {
        Self {
            cfg: VncAdapterMode::Socket(VncSocketConfig { stream }),
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
        verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> Box<dyn AdapterDisplay> {
        debug!("creating vnc adapter");
        let error_container: Rc<RefCell<Option<ConnectionError>>> = Rc::new(RefCell::new(None));

        let vnc = rdw_vnc::Display::new();

        vnc.connection().connect_vnc_error(glib::clone!(
            #[strong]
            error_container,
            move |_conn, err| Self::on_connect_vnc_error(&error_container, err)
        ));

        vnc.connection().connect_vnc_auth_failure(glib::clone!(
            #[strong]
            error_container,
            move |_conn, err| Self::on_vnc_auth_failure(&error_container, err)
        ));

        vnc.connection().connect_vnc_disconnected(glib::clone!(
            #[strong]
            error_container,
            #[strong]
            on_disconnected,
            move |_conn| Self::on_vnc_disconnected(&error_container, &*on_disconnected)
        ));

        vnc.connection()
            .connect_vnc_connected(move |_conn| Self::on_vnc_connected(&*on_connected));

        let (ca, host) = if let VncAdapterMode::Network(net_cfg) = &self.cfg {
            (net_cfg.ca.clone(), Some(net_cfg.host.clone()))
        } else {
            (None, None)
        };

        let user = self.user.clone();
        let password = self.password.clone();
        vnc.connection().connect_vnc_auth_credential(glib::clone!(
            #[strong]
            ca,
            move |conn, va| Self::on_vnc_auth_credential(
                conn,
                va,
                user.as_deref(),
                password.as_ref(),
                ca.as_deref()
            )
        ));

        glib::spawn_future_local(glib::clone!(
            #[strong]
            vnc,
            async move {
                if let Err(err) =
                    Self::verify_tls(&*verify_tls, ca.as_deref(), host.as_deref()).await
                {
                    on_disconnected(Err(err));
                    return;
                }
                if let Err(err) = self.cfg.connect(&vnc.connection()) {
                    warn!("connect failed: {err}");
                    on_disconnected(Err(ConnectionError::General(
                        Some(err.message.into_owned()),
                        anyhow!("vnc connect failed"),
                    )));
                }
            }
        ));

        Box::new(VncAdapterDisplay(vnc))
    }
}

impl VncAdapter {
    fn on_connect_vnc_error(error_container: &RefCell<Option<ConnectionError>>, err: &str) {
        warn!("VNC connect error: {:?}", &err);
        let err_msg = err.to_string();

        if error_container.borrow().is_none() {
            error_container.replace(Some(ConnectionError::General(
                Some(err_msg),
                anyhow!("{}", &err),
            )));
        }
    }
    fn on_vnc_auth_failure(error_container: &Rc<RefCell<Option<ConnectionError>>>, err: &str) {
        warn!("VNC auth failure: {:?}", &err);
        let err_msg = err.to_string();
        error_container.replace(Some(ConnectionError::AuthFailed(
            Some(err_msg),
            anyhow!("{}", &err),
        )));
    }
    fn on_vnc_disconnected(
        error_container: &Rc<RefCell<Option<ConnectionError>>>,
        on_disconnected: &dyn Fn(Result<(), ConnectionError>),
    ) {
        debug!("VNC connection disconnected");
        match error_container.borrow_mut().take() {
            None => on_disconnected(Ok(())),
            Some(err) => on_disconnected(Err(err)),
        }
    }
    fn on_vnc_connected(on_connected: &dyn Fn()) {
        debug!("VNC connection established");
        on_connected();
    }
    fn on_vnc_auth_credential(
        conn: &gvnc::Connection,
        va: &glib::ValueArray,
        user: Option<&str>,
        password: Option<&SecureString>,
        ca: Option<&str>,
    ) {
        debug!("VNC connection authenticating");
        let creds: Vec<_> = va
            .iter()
            .map(|v| v.get::<gvnc::ConnectionCredential>().unwrap())
            .collect();
        if creds.contains(&gvnc::ConnectionCredential::Username) {
            conn.set_credential(
                gvnc::ConnectionCredential::Username.into_glib(),
                user.unwrap_or_default(),
            )
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
                password.map(SecureString::unsecure).unwrap_or_default(),
            )
            .unwrap();
        }

        // TODO: gtk-vnc with this option is not released as stable yet, and we don't
        //       want to bother updating the Rust bindings to the unstable release,
        //       so we use this instead.
        //       In the future this will be gvnc::ConnectionCredential::CaCertData probably.
        const VNC_CONNECTION_CREDENTIAL_CA_CERT_DATA: i32 = 3;
        if let Some(ca) = ca
            && creds.contains(&gvnc::ConnectionCredential::__Unknown(
                VNC_CONNECTION_CREDENTIAL_CA_CERT_DATA,
            ))
        {
            debug!("providing CA cert");
            conn.set_credential(VNC_CONNECTION_CREDENTIAL_CA_CERT_DATA, ca)
                .unwrap();
        }
    }

    async fn verify_tls(
        verify_tls: &dyn Fn(VerifyTls) -> VerifyTlsResponse,
        ca: Option<&str>,
        host: Option<&str>,
    ) -> ConnectionResult<()> {
        let Some(ca) = ca else { return Ok(()) };
        let Some(host) = host else { return Ok(()) };
        let certs = match VerifiableCertChain::from_pem_chain(ca) {
            Ok(certs) => certs,
            Err(err) => {
                return Err(err);
            }
        };
        match verify_tls(VerifyTls::verify_async(certs, host, None, true)) {
            VerifyTlsResponse::Sync(_) => unreachable!(),
            VerifyTlsResponse::Async(fut) => {
                if !fut.await {
                    return Err(VerifyTls::error());
                }
            }
        }
        Ok(())
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
