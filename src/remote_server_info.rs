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
use crate::connection_loader::ConnectionLoader;
use crate::quick_connect;
use anyhow::anyhow;
use gettextrs::gettext;
use gtk::gio;
use gtk::prelude::*;
use libfieldmonitor::connection::{ConnectionError, ConnectionResult};
use std::borrow::Cow;

pub struct RemoteServerInfo<'a> {
    pub(crate) server_path: Cow<'a, str>,
    pub(crate) adapter_id: Cow<'a, str>,
    pub(crate) server_title: Cow<'a, str>,
    pub(crate) connection_title: Cow<'a, str>,
    pub(crate) loader: ConnectionLoader,
}

impl RemoteServerInfo<'static> {
    pub async fn try_from_file(
        file: gio::File,
        app: &FieldMonitorApplication,
        window: Option<&impl IsA<gtk::Window>>,
    ) -> ConnectionResult<Self> {
        // From URI scheme
        if file.uri_scheme().as_deref() != Some("file") {
            let info = quick_connect::try_from_uri(&file.uri(), app, window).await;
            return match info {
                None => Err(ConnectionError::General(
                    Some(gettext("Field Monitor does not support this URI scheme")),
                    anyhow!("Field Monitor does not support this URI scheme"),
                )),
                Some(Err(err)) => Err(err),
                Some(Ok(slf)) => Ok(slf),
            };
        }

        // From actual file
        match file.read_future(glib::Priority::DEFAULT).await {
            Ok(stream) => {
                let info = quick_connect::try_from_file(stream.into_read(), app, window).await;
                match info {
                    None => Err(ConnectionError::General(
                        Some(gettext("Unable to detect supported file format")),
                        anyhow!("failed to detect file format of any supported"),
                    )),
                    Some(Err(err)) => Err(err),
                    Some(Ok(slf)) => Ok(slf),
                }
            }
            Err(err) => Err(ConnectionError::General(
                Some(gettext("Field Monitor was unable to read the file")),
                err.into(),
            )),
        }
    }
}

impl<'a> RemoteServerInfo<'a> {
    pub fn new(
        server_path: Cow<'a, str>,
        adapter_id: Cow<'a, str>,
        server_title: Cow<'a, str>,
        connection_title: Cow<'a, str>,
        loader: ConnectionLoader,
    ) -> Self {
        Self {
            server_path,
            adapter_id,
            server_title,
            connection_title,
            loader,
        }
    }
}
