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
use std::rc::Rc;

use anyhow::anyhow;
use gettextrs::gettext;
use glib::prelude::*;
use log::{debug, warn};
use rdw_rdp::ironrdp::connector::ConnectorErrorKind;
use secure_string::SecureString;

use crate::adapter::types::{Adapter, AdapterDisplay, AdapterDisplayWidget};
use crate::connection::ConnectionError;

pub struct RdpAdapter {
    host: String,
    port: u16,
    user: String,
    password: SecureString,
}

impl RdpAdapter {
    pub const TAG: &'static str = "rdp";

    pub fn new(host: String, port: u32, user: String, password: SecureString) -> Self {
        Self {
            host,
            port: port as u16, // TODO: refactor
            user,
            password,
        }
    }

    pub fn label() -> Cow<'static, str> {
        gettext("RDP").into()
    }
}

impl Adapter for RdpAdapter {
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
    ) -> Box<dyn AdapterDisplay> {
        debug!("creating rdp adapter");
        let rdp = rdw_rdp::Display::new();

        let on_disconnected_cln = on_disconnected.clone();
        rdp.connect_rdp_connected_notify(move |rdp| {
            let connected = rdp.rdp_connected();
            if !connected {
                handle_rdp_error(None, &on_disconnected_cln);
            } else {
                debug!("RDP connection connected!");
                on_connected();
            }
        });

        glib::spawn_future_local(glib::clone!(
            #[weak]
            rdp,
            async move {
                let result = rdp
                    .rdp_connect(&self.host, self.port, &self.user, self.password.unsecure())
                    .await;

                if let Err(err) = result {
                    warn!("failed to connect rdp connection: {err}");
                    handle_rdp_error(Some(err), &on_disconnected);
                };
            }
        ));

        Box::new(RdpAdapterDisplay(rdp))
    }
}

fn handle_rdp_error(
    err: Option<rdw_rdp::Error>,
    on_disconnected: &Rc<dyn Fn(Result<(), ConnectionError>)>,
) {
    debug!("RDP connection disconnected (raw): {:?}", &err);
    match err {
        None => {
            debug!("RDP connection disconnected");
            on_disconnected(Ok(()))
        }
        Some(rdw_rdp::Error::Connector(err))
            if matches!(
                err.kind,
                ConnectorErrorKind::Credssp(_) | ConnectorErrorKind::AccessDenied
            ) =>
        {
            warn!("RDP connection auth error");
            on_disconnected(Err(ConnectionError::AuthFailed(
                None,
                anyhow!("RDP connection auth error"),
            )))
        }
        Some(err) => {
            warn!("RDP connection error: {:?}", err);
            on_disconnected(Err(ConnectionError::General(
                Some(format!("{}", err)),
                anyhow!(err),
            )))
        }
    }
}

pub struct RdpAdapterDisplay(rdw_rdp::Display);

impl AdapterDisplay for RdpAdapterDisplay {
    fn widget(&self) -> AdapterDisplayWidget {
        AdapterDisplayWidget::Rdw(self.0.clone().upcast())
    }

    fn close(&self) {
        let rdp = self.0.clone();
        glib::spawn_future_local(async move {
            rdp.rdp_disconnect().await.ok();
        });
    }
}

impl Drop for RdpAdapterDisplay {
    fn drop(&mut self) {
        self.close()
    }
}
