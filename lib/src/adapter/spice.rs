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
use log::debug;
use rdw_spice::spice;
use rdw_spice::spice::prelude::ChannelExt;
use rdw_spice::spice::ChannelEvent;
use secure_string::SecureString;

use crate::adapter::types::{Adapter, AdapterDisplay, AdapterDisplayWidget};
use crate::connection::ConnectionError;

pub struct SpiceAdapter {
    host: String,
    port: u32,
    user: String,
    password: SecureString,
}

impl SpiceAdapter {
    pub const TAG: &'static str = "spice";

    pub fn new(host: String, port: u32, user: String, password: SecureString) -> Self {
        Self {
            host,
            port,
            user,
            password,
        }
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
        let spice = rdw_spice::Display::new();

        let session = spice.session();

        session.set_uri(Some(&format!("spice://{}:{}", self.host, self.port)));
        session.set_username(Some(&self.user));
        session.set_password(Some(self.password.unsecure()));

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
                                    anyhow!("unknown error")
                                },
                            )))
                        }
                        ChannelEvent::ErrorAuth => {
                            on_disconnected_cln_cln(Err(ConnectionError::AuthFailed(
                                error.as_ref().map(ToString::to_string),
                                if let Some(e) = error {
                                    e.into()
                                } else {
                                    anyhow!("unknown error")
                                },
                            )))
                        }
                        _ => debug!("spice channel event: {event:?}"),
                    }
                });
            }
        });

        session.connect_disconnected(move |_| {
            // TODO: Error handling
            on_disconnected(Ok(()))
        });

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
