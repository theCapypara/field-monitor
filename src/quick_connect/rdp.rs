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
use crate::quick_connect::util::{parse_port, parse_query_args};
use crate::quick_connect::{OptionResult, QuickConnectAdapterType, QuickConnectConfig};
use crate::remote_server_info::RemoteServerInfo;
use anyhow::anyhow;
use fluent_uri::encoding::encoder::Query;
use fluent_uri::encoding::EStr;
use gettextrs::gettext;
use glib::object::IsA;
use libfieldmonitor::adapter::rdp::RdpAdapter;
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::connection::{ConnectionConfiguration, ConnectionError, ConnectionResult};
use secure_string::SecureString;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::convert::Infallible;
use std::hash::Hash;
use std::io::{Read, Seek};
use std::ops::Deref;
use std::str::FromStr;

pub async fn try_from_uri(
    uri: &str,
    app: &FieldMonitorApplication,
    window: Option<&impl IsA<gtk::Window>>,
) -> OptionResult<ConnectionResult<RemoteServerInfo<'static>>> {
    let Some(params) = uri.strip_prefix("rdp://") else {
        return Err(());
    };
    let Some(query) = parse_query_args(EStr::<Query>::new(params)) else {
        return Err(());
    };

    // https://learn.microsoft.com/de-de/windows-server/remote/remote-desktop-services/clients/remote-desktop-uri#legacy-rdp-uri-scheme
    // rdp://full%20address=s:mypc:3389&audiomode=i:2&disable%20themes=i:1

    Ok(quick_connect::construct(app, window, |config| {
        parse_rdp_file(config, &query)?;

        Ok(())
    })
    .await)
}

pub async fn try_from_file<T: Read + Seek>(
    mut stream: T,
    app: &FieldMonitorApplication,
    window: Option<&impl IsA<gtk::Window>>,
) -> Result<ConnectionResult<RemoteServerInfo<'static>>, T> {
    const LIMIT: usize = 128 * 1024;

    let mut buffer = Vec::with_capacity(LIMIT);
    let bytes = match (&mut stream)
        .take((LIMIT as u64) + 1)
        .read_to_end(&mut buffer)
    {
        Ok(v) => v,
        Err(err) => {
            return Ok(Err(ConnectionError::General(
                Some(gettext("Failed to read file")),
                err.into(),
            )))
        }
    };
    // We do not read files > 128ib.
    if bytes > LIMIT {
        return Err(stream);
    }
    let Ok(contents) = String::from_utf8(buffer) else {
        return Err(stream);
    };

    let Some(props) = contents
        .lines()
        .map(|line| {
            // https://learn.microsoft.com/en-us/azure/virtual-desktop/rdp-properties
            // full address:s:<hostname> or <IP Address>
            line.split_once(':')
        })
        .collect::<Option<HashMap<_, _>>>()
    else {
        return Err(stream);
    };

    Ok(quick_connect::construct(app, window, |config| {
        parse_rdp_file(config, &props)?;

        Ok(())
    })
    .await)
}

fn parse_rdp_file<S>(
    config: &mut ConnectionConfiguration,
    params: &HashMap<S, S>,
) -> Result<(), ConnectionError>
where
    S: Eq + Hash + Borrow<str> + Deref<Target = str>,
{
    config.set_adapter(QuickConnectAdapterType::Rdp);
    if let Some(value) = params.get("full address") {
        let value = strip_type_prefix('s', value)?;
        // Split at :, if not possible take as host and don't set port
        if let Some((host, port)) = value.split_once(':') {
            config.set_host(host);
            config.set_port(parse_port(port)?);
        } else {
            config.set_host(value);
        }
    } else {
        invalid()?;
    }
    if let Some(value) = params.get("username") {
        let value = strip_type_prefix('s', value)?;
        config.set_user(value);
    }
    // More options are not currently supported.

    // Non-standard title.
    if let Some(value) = params.get("title") {
        let value = strip_type_prefix('s', value)?;
        config.set_title(value);
    }
    Ok(())
}

fn strip_type_prefix(needle: char, haystack: &str) -> Result<&str, ConnectionError> {
    let mut chars = haystack.chars();
    if chars.next() != Some(needle) {
        invalid()?;
    }
    if chars.next() != Some(':') {
        invalid()?;
    }
    Ok(chars.as_str())
}

pub fn make_adapter(config: &ConnectionConfiguration) -> Box<dyn Adapter> {
    Box::new(RdpAdapter::new(
        config.host().to_string(),
        config.port().map(u32::from).unwrap_or(3389),
        config.user().map(ToString::to_string).unwrap_or_default(),
        config
            .password()
            .unwrap_or_else(|| SecureString::from_str("").unwrap()),
    ))
}

fn invalid() -> Result<Infallible, ConnectionError> {
    Err(ConnectionError::General(
        Some(gettext("Invalid RDP parameters")),
        anyhow!("invalid RDP parameters"),
    ))
}
