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
use crate::quick_connect::spice::SpiceQuickConnectConfig;
use crate::quick_connect::{QuickConnectAdapterType, QuickConnectConfig};
use crate::remote_server_info::RemoteServerInfo;
use glib::object::IsA;
use libfieldmonitor::connection::ConnectionResult;
use secure_string::SecureString;
use serde::Deserialize;
use std::io::{Read, Seek};
use std::num::NonZeroU32;
use std::str::FromStr;

// https://gitlab.com/virt-viewer/virt-viewer/-/blob/master/src/virt-viewer-file.c
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct VirtViewerFile {
    virt_viewer: VirtViewerSection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct VirtViewerSection {
    r#type: QuickConnectAdapterType,
    unix_path: Option<String>,
    host: Option<String>,
    port: Option<NonZeroU32>,
    tls_port: Option<NonZeroU32>,
    username: Option<String>,
    password: Option<String>,
    ca: Option<String>,
    host_subject: Option<String>,
    title: Option<String>,
    proxy: Option<String>,
}

pub async fn try_from_file<T: Read + Seek>(
    mut stream: T,
    app: &FieldMonitorApplication,
    window: Option<&impl IsA<gtk::Window>>,
) -> Result<ConnectionResult<RemoteServerInfo<'static>>, T> {
    let Ok(vv): Result<VirtViewerFile, _> = serde_ini::from_read(&mut stream) else {
        return Err(stream);
    };
    let vv = vv.virt_viewer;

    Ok(quick_connect::construct(app, window, |config| {
        config.set_adapter(vv.r#type);
        if let Some(title) = &vv.title {
            config.set_title(title);
        }

        if let Some(host) = &vv.host {
            config.set_host(host);
            if config.title().is_empty() {
                config.set_title(host);
            }
        }

        if let Some(port) = vv.port {
            config.set_port(port);
        } else if let Some(tls_port) = vv.tls_port {
            if !matches!(vv.r#type, QuickConnectAdapterType::Spice) {
                config.set_port(tls_port);
            }
        }

        if let Some(user) = &vv.username {
            config.set_user(user);
        }
        if let Some(password) = &vv.password {
            config.set_password(SecureString::from_str(password).unwrap());
        }

        match vv.r#type {
            QuickConnectAdapterType::Spice => {
                if let Some(ca) = &vv.ca {
                    config.set_spice_ca(ca);
                }
                if let Some(host_subject) = &vv.host_subject {
                    config.set_spice_cert_subject(host_subject);
                }
                if let Some(tls_port) = vv.tls_port {
                    config.set_spice_tls_port(tls_port);
                }
                if let Some(proxy) = &vv.proxy {
                    config.set_spice_proxy(proxy);
                }
                if let Some(unix_path) = &vv.unix_path {
                    config.set_spice_unix_path(unix_path);

                    if config.title().is_empty() {
                        config.set_title(unix_path);
                    }
                }
            }
            QuickConnectAdapterType::Rdp => {}
            QuickConnectAdapterType::Vnc => {}
        }
        Ok(())
    })
    .await)
}
