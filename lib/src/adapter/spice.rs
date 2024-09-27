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

use gettextrs::gettext;
use glib::prelude::*;
use rdw_spice::spice;
use rdw_spice::spice::prelude::ChannelExt;

use crate::adapter::types::{Adapter, AdapterDisplay};
use crate::connection::ConnectionError;

pub struct SpiceAdapter {
    host: String,
    port: u32,
    password: String,
}

impl SpiceAdapter {
    pub const TAG: &'static str = "spice";

    pub fn new(host: String, port: u32, password: String) -> Self {
        Self {
            host,
            port,
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
    ) -> AdapterDisplay {
        let spice = rdw_spice::Display::new();

        let session = spice.session();

        session.set_uri(Some(&format!("spice://{}:{}", self.host, self.port)));

        let on_disconnected_cln = on_disconnected.clone();
        session.connect_channel_new(move |_, channel| {
            if let Ok(main) = channel.clone().downcast::<spice::MainChannel>() {
                let on_disconnected_cln_cln = on_disconnected_cln.clone();
                main.connect_channel_event(move |channel, event| {
                    use spice::ChannelEvent::*;
                    if event == ErrorConnect {
                        if let Some(err) = channel.error() {
                            on_disconnected_cln_cln(Err(ConnectionError::General(
                                Some(err.to_string()),
                                err.into(),
                            )))
                        }
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

        AdapterDisplay::Rdw(spice.upcast())
    }
}
