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
use std::borrow::Cow;
use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use anyhow::anyhow;
use derive_builder::Builder;
use gettextrs::gettext;
use glib::prelude::*;
use log::debug;
use rdw_spice::spice;
use rdw_spice::spice::prelude::ChannelExt;
use rdw_spice::spice::{ChannelEvent, Session};
use secure_string::SecureString;

use crate::adapter::types::{Adapter, AdapterDisplay, AdapterDisplayWidget};
use crate::connection::ConnectionError;

#[derive(Builder, Debug, Clone, Default)]
#[builder(pattern = "owned")]
#[non_exhaustive]
pub struct SpiceSessionConfig {
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

impl SpiceSessionConfig {
    fn apply(self, session: &mut Session) {
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
}

pub struct SpiceAdapter(SpiceSessionConfig);

impl SpiceAdapter {
    pub const TAG: &'static str = "spice";

    pub fn new(host: String, port: u32, user: String, password: SecureString) -> Self {
        Self(SpiceSessionConfig {
            uri: Some(format!("spice://{}:{}", host, port)),
            username: Some(user),
            password: Some(password),
            ca: None,
            host: None,
            port: None,
            cert_subject: None,
            tls_port: None,
            proxy: None,
            unix_path: None,
        })
    }

    pub fn new_with_custom_config(config: SpiceSessionConfig) -> Self {
        Self(config)
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
    ) -> Box<dyn AdapterDisplay> {
        debug!("creating spice adapter");
        let spice = rdw_spice::Display::new();

        let mut session = spice.session();
        self.0.apply(&mut session);

        let disconnect_error: Rc<RefCell<Option<glib::Error>>> = Default::default();

        let on_disconnected_cln = on_disconnected.clone();
        session.connect_channel_new(move |_, channel| {
            if let Ok(main) = channel.clone().downcast::<spice::MainChannel>() {
                let on_disconnected_cln_cln = on_disconnected_cln.clone();
                main.connect_channel_event(move |channel, event| {
                    let error = channel.error();
                    match event {
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
                    on_disconnected(Err(con_error))
                } else {
                    on_disconnected(Ok(()))
                }
            }
        ));

        glib::spawn_future_local(async move {
            session.connect();
            on_connected();
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
