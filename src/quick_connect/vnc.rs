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
use crate::application::FieldMonitorApplication;
use crate::quick_connect;
use crate::quick_connect::util::{parse_port, parse_query_args, parse_userinfo, set_title_in};
use crate::quick_connect::{OptionResult, QuickConnectAdapterType, QuickConnectConfig};
use crate::remote_server_info::RemoteServerInfo;
use anyhow::anyhow;
use fluent_uri::Uri;
use gettextrs::gettext;
use glib::object::IsA;
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::connection::{ConnectionConfiguration, ConnectionError, ConnectionResult};
use secure_string::SecureString;
use std::convert::Infallible;
use std::str::FromStr;

pub async fn try_from_uri(
    uri: &Uri<&str>,
    app: &FieldMonitorApplication,
    window: Option<&impl IsA<gtk::Window>>,
) -> OptionResult<ConnectionResult<RemoteServerInfo<'static>>> {
    if uri.scheme().as_str() != "vnc" {
        return Err(());
    }
    // https://www.rfc-editor.org/rfc/rfc7869.html

    // vnc-uri = "vnc://" [ userinfo "@" ] [ host [ ":" port ] ]
    //              [ "?" [ vnc-params ] ]
    // vnc://host:port?param1=value1&param2=value2...
    Ok(quick_connect::construct(app, window, |config| {
        config.set_adapter(QuickConnectAdapterType::Vnc);
        // Authority
        if let Some(authority) = uri.authority() {
            if authority.host().is_empty() {
                invalid_uri()?;
            }
            config.set_host(authority.host());
            if let Some(port) = authority.port() {
                config.set_port(parse_port(port.as_str())?);
            }
            let (user, pass) = parse_userinfo(authority.userinfo());
            if let Some(user) = user {
                if !user.is_empty() {
                    config.set_user(&user.decode().into_string_lossy());
                }
            }
            if let Some(pass) = pass {
                config.set_password(
                    SecureString::from_str(&pass.decode().into_string_lossy()).unwrap(),
                );
            }
        }

        // Arguments
        let mut has_title = false;
        let query_args = parse_query_args(uri.query());
        if let Some(query) = &query_args {
            if let Some(name) = query.get("ConnectionName") {
                config.set_title(name);
                has_title = true;
            }
            if let Some(username) = query.get("VncUsername") {
                config.set_user(username);
            }
            if let Some(password) = query.get("VncPassword") {
                config.set_password(SecureString::from_str(password).unwrap());
            }
            // More options are not currently supported.
        }

        if !has_title {
            // non-standard ?title=
            set_title_in(uri, query_args, config);
        }

        Ok(())
    })
    .await)
}

pub fn make_adapter(config: &ConnectionConfiguration) -> Box<dyn Adapter> {
    Box::new(VncAdapter::new(
        config.host().to_string(),
        config.port().map(u32::from).unwrap_or(5900),
        config.user().map(ToString::to_string).unwrap_or_default(),
        config
            .password()
            .unwrap_or_else(|| SecureString::from_str("").unwrap()),
    ))
}

fn invalid_uri() -> Result<Infallible, ConnectionError> {
    Err(ConnectionError::General(
        Some(gettext("Invalid VNC URI")),
        anyhow!("invalid VNC URI"),
    ))
}
