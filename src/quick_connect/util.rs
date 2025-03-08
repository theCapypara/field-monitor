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
use crate::quick_connect::QuickConnectConfig;
use fluent_uri::encoding::encoder::{Query, Userinfo};
use fluent_uri::encoding::EStr;
use fluent_uri::{Builder, Uri, UriRef};
use gettextrs::gettext;
use libfieldmonitor::connection::{ConnectionConfiguration, ConnectionError};
use std::borrow::Cow;
use std::collections::HashMap;
use std::num::NonZeroU32;

pub(super) fn parse_query_args(query: Option<&EStr<Query>>) -> Option<HashMap<Cow<str>, Cow<str>>> {
    query.map(|query| {
        query
            .split('&')
            .map(|s| s.split_once('=').unwrap_or((s, EStr::EMPTY)))
            .map(|(k, v)| {
                (
                    k.decode().into_string_lossy(),
                    v.decode().into_string_lossy(),
                )
            })
            .collect()
    })
}

pub(super) fn parse_userinfo(
    userinfo: Option<&EStr<Userinfo>>,
) -> (Option<&EStr<Userinfo>>, Option<&EStr<Userinfo>>) {
    match userinfo {
        None => (None, None),
        Some(userinfo) => match userinfo.split_once(':') {
            None => (Some(userinfo), None),
            Some((user, pass)) => (Some(user), Some(pass)),
        },
    }
}

pub(super) fn parse_port(port: impl AsRef<str>) -> Result<NonZeroU32, ConnectionError> {
    match port.as_ref().parse::<u32>() {
        Ok(port) => match NonZeroU32::try_from(port) {
            Ok(port) => Ok(port),
            Err(err) => Err(ConnectionError::General(
                Some(gettext("Invalid port in URI")),
                err.into(),
            )),
        },
        Err(err) => Err(ConnectionError::General(
            Some(gettext("Invalid port in URI")),
            err.into(),
        )),
    }
}

pub(super) fn set_title_in(
    uri: &Uri<&str>,
    query: Option<HashMap<Cow<str>, Cow<str>>>,
    config: &mut ConnectionConfiguration,
) {
    if let Some(query) = query {
        if let Some(title) = query.get("title") {
            config.set_title(title);
        }
    }

    let mut host = "";
    let mut port = None;
    if let Some(a) = uri.authority() {
        host = a.host();
        port = a.port();
    }

    if config.title().is_empty() {
        let uri = UriRef::builder()
            .scheme(uri.scheme())
            .authority_with(|b| {
                b.optional(Builder::userinfo, None)
                    .host(EStr::new_or_panic(host))
                    .optional(Builder::port, port)
            })
            .path(EStr::new_or_panic(""))
            .optional(Builder::query, None)
            .optional(Builder::fragment, None)
            .build()
            .unwrap();
        config.set_title(uri.as_ref());
    }
}
