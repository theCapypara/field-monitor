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
use crate::cert_security::{
    VerifiableCertChain, VerifyTls, VerifyTlsResponse, extract_common_name,
};
use crate::connection::ConnectionError;
use anyhow::anyhow;
use derive_builder::Builder;
use gettextrs::gettext;
use glib::prelude::*;
use log::{debug, error, warn};
use rdw_spice::spice;
use rdw_spice::spice::prelude::ChannelExt;
use rdw_spice::spice::{ChannelEvent, Session};
use secure_string::SecureString;
use std::borrow::Cow;
use std::cell::RefCell;
use std::mem;
use std::num::NonZeroU32;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use x509_cert::name::Name;

pub type MakeChannelSocket = Box<dyn Fn() -> anyhow::Result<UnixStream> + Send + Sync>;

#[derive(Builder, Debug, Clone, Default)]
#[builder(pattern = "owned")]
#[non_exhaustive]
pub struct SpiceNetworkSessionConfig {
    #[builder(default = "None")]
    uri: Option<String>,
    #[builder(default = "None")]
    username: Option<String>,
    #[builder(default = "None")]
    password: Option<SecureString>,
    #[builder(default = "None")]
    ca: Option<Vec<u8>>,
    #[builder(default = "None")]
    host: Option<String>,
    #[builder(default = "None")]
    port: Option<NonZeroU32>,
    #[builder(default = "None")]
    cert_subject: Option<String>,
    #[builder(default = "None")]
    tls_port: Option<NonZeroU32>,
    #[builder(default = "None")]
    proxy: Option<String>,
    #[builder(default = "None")]
    unix_path: Option<String>,
}

trait MakeSession {
    fn apply(&self, session: &Session);
    fn connect(self, session: &Session) -> bool;
}

impl MakeSession for SpiceNetworkSessionConfig {
    fn apply(&self, session: &Session) {
        // We check for Some, because the bindings seem to have some bugs / weird behaviour
        // with None values.
        if self.uri.is_some() {
            session.set_uri(self.uri.as_deref());
        }
        if self.username.is_some() {
            session.set_username(self.username.as_deref());
        }
        if self.password.is_some() {
            session.set_password(self.password.as_ref().map(|v| v.unsecure()));
        }
        if self.ca.is_some() {
            session.set_ca(self.ca.as_ref().map(Into::into).as_ref());
        }
        if self.host.is_some() {
            session.set_host(self.host.as_deref());
        }
        if self.port.is_some() {
            session.set_port(self.port.map(|v| v.to_string()).as_deref());
        }
        if self.cert_subject.is_some() {
            session.set_cert_subject(self.cert_subject.as_deref());
        }
        if self.tls_port.is_some() {
            session.set_tls_port(self.tls_port.map(|v| v.to_string()).as_deref());
        }
        if self.proxy.is_some() {
            session.set_proxy(self.proxy.as_deref());
        }
        if self.unix_path.is_some() {
            session.set_unix_path(self.unix_path.as_deref());
        }
    }

    fn connect(self, session: &Session) -> bool {
        session.connect()
    }
}

struct SpiceSocketSessionConfig {
    stream: UnixStream,
    channel_stream_fn: Arc<MakeChannelSocket>,
    username: Option<String>,
    password: Option<SecureString>,
}

impl MakeSession for SpiceSocketSessionConfig {
    fn apply(&self, session: &Session) {
        if self.username.is_some() {
            session.set_username(self.username.as_deref());
        }
        if self.password.is_some() {
            session.set_password(self.password.as_ref().map(|v| v.unsecure()));
        }
        session.set_client_sockets(true);
    }

    fn connect(self, session: &Session) -> bool {
        if session.open_fd(self.stream.as_raw_fd()) {
            mem::forget(self.stream);
            true
        } else {
            false
        }
    }
}

enum SpiceAdapterMode {
    /// The connection is made over the network.
    Network(SpiceNetworkSessionConfig),
    /// The connection is made using a socket
    Socket(SpiceSocketSessionConfig),
}

impl MakeSession for SpiceAdapterMode {
    fn apply(&self, session: &Session) {
        match self {
            SpiceAdapterMode::Network(s) => s.apply(session),
            SpiceAdapterMode::Socket(s) => s.apply(session),
        }
    }

    fn connect(self, session: &Session) -> bool {
        match self {
            SpiceAdapterMode::Network(s) => s.connect(session),
            SpiceAdapterMode::Socket(s) => s.connect(session),
        }
    }
}

pub struct SpiceAdapter(SpiceAdapterMode);

impl SpiceAdapter {
    pub const TAG: &'static str = "spice";

    pub fn new(
        host: String,
        port: Option<NonZeroU32>,
        tls_port: Option<NonZeroU32>,
        user: String,
        password: SecureString,
    ) -> Self {
        Self(SpiceAdapterMode::Network(SpiceNetworkSessionConfig {
            uri: None,
            username: Some(user),
            password: Some(password),
            ca: None,
            host: Some(host),
            port,
            cert_subject: None,
            tls_port,
            proxy: None,
            unix_path: None,
        }))
    }

    pub fn new_with_custom_config(config: SpiceNetworkSessionConfig) -> Self {
        Self(SpiceAdapterMode::Network(config))
    }

    pub fn new_from_socket(
        stream: UnixStream,
        make_channel_socket: MakeChannelSocket,
        username: Option<String>,
        password: Option<SecureString>,
    ) -> Self {
        Self(SpiceAdapterMode::Socket(SpiceSocketSessionConfig {
            stream,
            channel_stream_fn: Arc::new(make_channel_socket),
            username,
            password,
        }))
    }

    pub fn label() -> Cow<'static, str> {
        gettext("SPICE").into()
    }
}

impl Adapter for SpiceAdapter {
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
        verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> Box<dyn AdapterDisplay> {
        debug!("creating spice adapter");
        let spice = rdw_spice::Display::new();

        let session = spice.session();
        self.0.apply(&session);
        let channel_stream_fn = match &self.0 {
            SpiceAdapterMode::Socket(cfg) => Some(cfg.channel_stream_fn.clone()),
            _ => None,
        };

        let disconnect_error: Rc<RefCell<Option<glib::Error>>> = Default::default();

        let on_disconnected_cln = on_disconnected.clone();
        session.connect_channel_new(move |_, channel| {
            debug!("channel-new: {}", channel.type_().name());
            // Open channel fd if we are connected using a socket.
            if let Some(open_stream_fn) = channel_stream_fn.clone() {
                let on_disconnected = on_disconnected_cln.clone();
                debug!("connecting open fd");
                channel.connect_open_fd(move |channel, _| {
                    match open_stream_fn() {
                        Ok(stream) => {
                            debug!("connecting channel {channel:?} with {stream:?}");
                            if !channel.open_fd(stream.as_raw_fd()) {
                                error!("failed to open channel using fd (open_fd)");
                                on_disconnected(Err(ConnectionError::General(
                                    None,
                                    anyhow!("failed to open channel using fd"),
                                )));
                            } else {
                                mem::forget(stream);
                            }
                        }
                        Err(err) => {
                            error!("failed to open channel using fd (open_stream_fn)");
                            on_disconnected(Err(ConnectionError::General(
                                None,
                                anyhow!("failed to open channel using fd: {err}"),
                            )));
                        }
                    };
                });
            };

            if let Ok(main) = channel.clone().downcast::<spice::MainChannel>() {
                let on_disconnected_cln_cln = on_disconnected_cln.clone();
                let on_connected_cln = on_connected.clone();
                main.connect_channel_event(move |channel, event| {
                    let error = channel.error();
                    match event {
                        ChannelEvent::Opened => {
                            debug!("main channel opened");
                            on_connected_cln();
                        }
                        ChannelEvent::ErrorConnect
                        | ChannelEvent::ErrorTls
                        | ChannelEvent::ErrorLink
                        | ChannelEvent::ErrorIo => {
                            on_disconnected_cln_cln(Err(ConnectionError::General(
                                error.as_ref().map(ToString::to_string),
                                if let Some(e) = error {
                                    e.into()
                                } else {
                                    anyhow!("unknown error for event {event:?}")
                                },
                            )))
                        }
                        ChannelEvent::ErrorAuth => {
                            on_disconnected_cln_cln(Err(ConnectionError::AuthFailed(
                                error.as_ref().map(ToString::to_string),
                                if let Some(e) = error {
                                    e.into()
                                } else {
                                    anyhow!("unknown error for event {event:?}")
                                },
                            )))
                        }
                        _ => debug!("spice channel event: {event:?}"),
                    }
                });
            }
        });
        session.connect_channel_destroy(glib::clone!(
            #[strong]
            disconnect_error,
            move |_, channel| {
                if let Some(error) = channel.error() {
                    disconnect_error.replace(Some(error));
                }
            }
        ));

        let on_disconnected_cln = on_disconnected.clone();
        session.connect_disconnected(glib::clone!(
            #[strong]
            disconnect_error,
            move |_| {
                if let Some(error) = disconnect_error.take() {
                    let con_error = if let Some(error) = error.kind::<spice::ClientError>() {
                        match error {
                            spice::ClientError::AuthNeedsPassword
                            | spice::ClientError::AuthNeedsUsername
                            | spice::ClientError::AuthNeedsPasswordAndUsername => {
                                ConnectionError::AuthFailed(None, anyhow!("auth failed"))
                            }
                            _ => ConnectionError::General(None, anyhow!("{:?}", error)),
                        }
                    } else {
                        ConnectionError::General(None, anyhow::Error::from(Box::new(error)))
                    };
                    on_disconnected_cln(Err(con_error))
                } else {
                    on_disconnected_cln(Ok(()))
                }
            }
        ));

        let session_cfg = self.0;
        glib::spawn_future_local(async move {
            // TLS verification
            if let Some(ca) = session.ca() {
                let certs = match VerifiableCertChain::from_pem_chain(ca) {
                    Ok(certs) => certs,
                    Err(err) => {
                        return on_disconnected(Err(err));
                    }
                };
                let subject_line = match session
                    .cert_subject()
                    .map(|x| Name::from_str(x.as_str()))
                    .transpose()
                {
                    Ok(subject_line) => subject_line,
                    Err(err) => {
                        warn!("failed to parse cert name / subject: {:?}", err);
                        return on_disconnected(Err(ConnectionError::General(None, err.into())));
                    }
                };
                match verify_tls(VerifyTls::verify_async(
                    certs,
                    subject_line
                        .as_ref()
                        .and_then(extract_common_name)
                        .or_else(|| session.host().as_deref().map(ToString::to_string))
                        .unwrap_or_default(),
                    subject_line,
                    true,
                )) {
                    VerifyTlsResponse::Sync(_) => unreachable!(),
                    VerifyTlsResponse::Async(fut) => {
                        if !fut.await {
                            return on_disconnected(Err(VerifyTls::error()));
                        }
                    }
                }
            }

            // Connect
            if !session_cfg.connect(&session) {
                // handled by disconnected signal.
                warn!("connect failed");
            }
        });

        Box::new(SpiceAdapterDisplay(spice))
    }
}

pub struct SpiceAdapterDisplay(rdw_spice::Display);

impl AdapterDisplay for SpiceAdapterDisplay {
    fn widget(&self) -> AdapterDisplayWidget {
        AdapterDisplayWidget::Rdw(self.0.clone().upcast())
    }

    fn close(&self) {
        self.0.session().disconnect();
    }
}

impl Drop for SpiceAdapterDisplay {
    fn drop(&mut self) {
        self.close()
    }
}
