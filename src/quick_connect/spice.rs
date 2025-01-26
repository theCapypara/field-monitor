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
use crate::application::FieldMonitorApplication;
use crate::quick_connect;
use crate::quick_connect::util::{parse_port, parse_query_args, parse_userinfo, set_title_in};
use crate::quick_connect::{OptionResult, QuickConnectAdapterType, QuickConnectConfig};
use crate::remote_server_info::RemoteServerInfo;
use anyhow::anyhow;
use fluent_uri::Uri;
use gettextrs::gettext;
use glib::object::IsA;
use libfieldmonitor::adapter::spice::{SpiceAdapter, SpiceSessionConfigBuilder};
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::connection::{
    ConfigAccess, ConfigAccessMut, ConnectionConfiguration, ConnectionError, ConnectionResult,
};
use secure_string::SecureString;
use std::convert::Infallible;
use std::num::NonZeroU32;
use std::str::FromStr;

#[allow(unused)]
pub trait SpiceQuickConnectConfig {
    fn spice_uri(&self) -> Option<&str>;
    fn set_spice_uri(&mut self, value: &str);
    fn spice_ca(&self) -> Option<&str>;
    fn set_spice_ca(&mut self, value: &str);
    fn spice_cert_subject(&self) -> Option<&str>;
    fn set_spice_cert_subject(&mut self, value: &str);
    fn spice_tls_port(&self) -> Option<NonZeroU32>;
    fn set_spice_tls_port(&mut self, value: NonZeroU32);
    fn spice_proxy(&self) -> Option<&str>;
    fn set_spice_proxy(&mut self, value: &str);
    fn spice_unix_path(&self) -> Option<&str>;
    fn set_spice_unix_path(&mut self, value: &str);
}

impl SpiceQuickConnectConfig for ConnectionConfiguration {
    fn spice_uri(&self) -> Option<&str> {
        self.get_try_as_str("spice-uri")
    }

    fn set_spice_uri(&mut self, value: &str) {
        self.set_value("spice-uri", value);
    }

    fn spice_ca(&self) -> Option<&str> {
        self.get_try_as_str("spice-ca")
    }

    fn set_spice_ca(&mut self, value: &str) {
        self.set_value("spice-ca", value);
    }

    fn spice_cert_subject(&self) -> Option<&str> {
        self.get_try_as_str("spice-cert-subject")
    }

    fn set_spice_cert_subject(&mut self, value: &str) {
        self.set_value("spice-cert-subject", value);
    }

    fn spice_tls_port(&self) -> Option<NonZeroU32> {
        self.get_try_as_u64("spice-tls-port").and_then(|v| {
            if v <= (u32::MAX as u64) {
                NonZeroU32::new(v as u32)
            } else {
                None
            }
        })
    }

    fn set_spice_tls_port(&mut self, value: NonZeroU32) {
        self.set_value("spice-tls-port", value.get());
    }

    fn spice_proxy(&self) -> Option<&str> {
        self.get_try_as_str("spice-proxy")
    }

    fn set_spice_proxy(&mut self, value: &str) {
        self.set_value("spice-proxy", value);
    }

    fn spice_unix_path(&self) -> Option<&str> {
        self.get_try_as_str("spice-unix-path")
    }

    fn set_spice_unix_path(&mut self, value: &str) {
        self.set_value("spice-unix-path", value);
    }
}

pub async fn try_from_uri(
    uri: &Uri<&str>,
    app: &FieldMonitorApplication,
    window: Option<&impl IsA<gtk::Window>>,
) -> OptionResult<ConnectionResult<RemoteServerInfo<'static>>> {
    // https://gitlab.com/libvirt/libvirt/-/blob/master/tools/virsh-domain.c#L11560-11698
    // https://gitlab.freedesktop.org/spice/spice-gtk/-/blob/master/src/spice-session.c#L392-601

    match uri.scheme().as_str() {
        "spice+unix" => Ok(quick_connect::construct(app, window, |config| {
            config.set_adapter(QuickConnectAdapterType::Spice);

            // spice+unix:///xyz.socket

            if let Some(authority) = uri.authority() {
                if !authority.host().is_empty() || authority.has_userinfo() || authority.has_port()
                {
                    invalid_uri()?;
                }
            }

            if uri.path().is_empty() {
                invalid_uri()?;
            }
            config.set_spice_unix_path(&uri.path().decode().into_string_lossy());

            // non-standard ?title=
            set_title_in(uri, parse_query_args(uri.query()), config);

            Ok(())
        })
        .await),
        "spice" | "spice+tls" => {
            Ok(quick_connect::construct(app, window, |config| {
                config.set_adapter(QuickConnectAdapterType::Spice);

                // spice://server:1234?tlsPort=4567&password=foo
                // spice://user:pass@server:4567?tlsPort=1234
                // spice+tls://user:pass@server:1234

                if let Some(authority) = uri.authority() {
                    if authority.host().is_empty() {
                        invalid_uri()?;
                    }
                    config.set_host(authority.host());
                    if let Some(port) = authority.port() {
                        let port = parse_port(port.as_str())?;
                        if uri.scheme().as_str() == "spice+tls" {
                            config.set_spice_tls_port(port);
                        } else {
                            config.set_port(port);
                        }
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

                let query_args = parse_query_args(uri.query());
                if let Some(query) = &query_args {
                    if let Some(tls_port) = query.get("tlsPort") {
                        config.set_spice_tls_port(parse_port(tls_port)?);
                    }
                    if let Some(tls_port) = query.get("tls-port") {
                        config.set_spice_tls_port(parse_port(tls_port)?);
                    }
                    if let Some(tls_port) = query.get("tls_port") {
                        config.set_spice_tls_port(parse_port(tls_port)?);
                    }
                    if let Some(username) = query.get("username") {
                        config.set_user(username);
                    }
                    if let Some(password) = query.get("password") {
                        config.set_password(SecureString::from_str(password).unwrap());
                    }
                }

                // non-standard ?title=
                set_title_in(uri, query_args, config);

                Ok(())
            })
            .await)
        }
        _ => Err(()),
    }
}

fn invalid_uri() -> Result<Infallible, ConnectionError> {
    Err(ConnectionError::General(
        Some(gettext("Invalid SPICE URI")),
        anyhow!("invalid SPICE URI"),
    ))
}

pub fn make_adapter(config: &ConnectionConfiguration) -> Box<dyn Adapter> {
    Box::new(SpiceAdapter::new_with_custom_config(
        SpiceSessionConfigBuilder::default()
            .uri(config.spice_uri().map(ToString::to_string))
            .username(config.user().map(ToString::to_string))
            .password(config.password())
            .ca(config
                .spice_ca()
                .map(|s| s.replace(r"\n", "\n"))
                .map(String::into_bytes))
            .host(Some(config.host().to_string()))
            .port(config.port())
            .cert_subject(config.spice_cert_subject().map(ToString::to_string))
            .tls_port(config.spice_tls_port())
            .proxy(config.spice_proxy().map(ToString::to_string))
            .unix_path(config.spice_unix_path().map(ToString::to_string))
            .build()
            .unwrap(),
    ))
}
