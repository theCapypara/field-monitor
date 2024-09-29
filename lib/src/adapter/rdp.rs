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
use glib::clone;
use glib::prelude::*;
use log::{debug, warn};
use rdw_rdp::freerdp::{RdpCode, RdpErr, RdpErrConnect};
use secure_string::SecureString;

use crate::adapter::types::{Adapter, AdapterDisplay};
use crate::connection::ConnectionError;

pub struct RdpAdapter {
    host: String,
    port: u32,
    user: String,
    password: SecureString,
}

impl RdpAdapter {
    pub const TAG: &'static str = "rdp";

    pub fn new(host: String, port: u32, user: String, password: SecureString) -> Self {
        Self {
            host,
            port,
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
    ) -> AdapterDisplay {
        let rdp = rdw_rdp::Display::new();

        let settings_result = rdp.with_settings(|s| {
            s.set_server_port(self.port);
            s.set_server_hostname(Some(self.host.as_str()))?;
            s.set_username(Some(self.user.as_str()))?;
            s.set_password(Some(self.password.unsecure()))?;
            s.set_remote_fx_codec(true);
            s.parse_command_line(&["field-monitor", "/rfx"], true)?;
            Ok(())
        });

        if let Err(err) = settings_result {
            on_disconnected(Err(ConnectionError::General(
                Some(gettext("Failed to process RDP connection configuration")),
                anyhow::Error::new(err),
            )));
        };

        let on_disconnected_cln = on_disconnected.clone();
        rdp.connect_rdp_connected_notify(move |rdp| {
            let connected = rdp.rdp_connected();
            if !connected {
                handle_rdp_error(rdp, &on_disconnected_cln);
            } else {
                debug!("RDP connection connected!");
                on_connected();
            }
        });

        glib::spawn_future_local(clone!(
            #[weak]
            rdp,
            async move {
                if rdp.rdp_connect().await.is_err() {
                    handle_rdp_error(&rdp, &on_disconnected);
                }
            }
        ));

        AdapterDisplay::Rdw(rdp.upcast())
    }
}

fn handle_rdp_error(
    rdp: &rdw_rdp::Display,
    on_disconnected: &Rc<dyn Fn(Result<(), ConnectionError>)>,
) {
    let err = rdp.last_error();
    debug!("RDP connection disconnected (raw): {:?}", &err);
    match err {
        None => {
            debug!("RDP connection disconnected");
            on_disconnected(Ok(()))
        }
        Some(RdpErr::RdpErrConnect(RdpErrConnect::AuthenticationFailed)) => {
            warn!("RDP connection auth error");
            on_disconnected(Err(ConnectionError::AuthFailed(
                None,
                anyhow!("RDP connection auth error"),
            )))
        }
        Some(err) => {
            warn!("RDP connection error: {:?}", err);
            let dbg_err = format!("{:?}", err);
            let err_code = match err {
                RdpErr::RdpErrBase(err) => err as u32,
                RdpErr::RdpErrInfo(err) => err as u32,
                RdpErr::RdpErrConnect(err) => err as u32,
            };
            on_disconnected(Err(ConnectionError::General(
                Some(format!("{}", RdpCode(err_code))),
                anyhow!("{:?}: {}", dbg_err, RdpCode(err_code)),
            )))
        }
    }
}
