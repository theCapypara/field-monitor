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

use adw::prelude::Cast;
use gettextrs::gettext;

use crate::adapter::types::Adapter;
use crate::application::FieldMonitorApplication;
use crate::connection::types::*;
use crate::connection::vnc::credential_preferences::VncCredentialPreferences;
use crate::connection::vnc::preferences::VncPreferences;

mod credential_preferences;
mod preferences;

pub struct VncConnectionProviderConstructor;

impl ConnectionProviderConstructor for VncConnectionProviderConstructor {
    fn tag(&self) -> &'static str {
        "vnc"
    }

    fn new(&self, app: &FieldMonitorApplication) -> Box<dyn ConnectionProvider> {
        Box::new(VncConnectionProvider { app: app.clone() })
    }
}

pub struct VncConnectionProvider {
    app: FieldMonitorApplication,
}

impl ConnectionProvider for VncConnectionProvider {
    fn title(&self) -> Cow<str> {
        gettext("VNC Connection").into()
    }

    fn title_plural(&self) -> Cow<str> {
        gettext("VNC Connections").into()
    }

    fn add_title(&self) -> Cow<str> {
        gettext("Add VNC Connection").into()
    }

    fn description(&self) -> Cow<str> {
        gettext("Setup a connection to a single VNC server.").into()
    }

    fn preferences(&self, configuration: Option<&ConnectionConfiguration>) -> gtk::Widget {
        VncPreferences::new(configuration).upcast()
    }

    fn create_connection(
        &self,
        preferences: &gtk::Widget,
    ) -> anyhow::Result<ConnectionConfiguration> {
        let preferences = preferences
            .downcast_ref::<VncPreferences>()
            .expect("create_connection got invalid widget type");
        todo!()
    }

    fn update_connection(
        &self,
        preferences: &gtk::Widget,
        configuration: &mut ConnectionConfiguration,
    ) -> anyhow::Result<()> {
        let preferences = preferences
            .downcast_ref::<VncPreferences>()
            .expect("update_connection got invalid widget type");
        todo!()
    }

    fn configure_credentials(&self, configuration: &ConnectionConfiguration) -> gtk::Widget {
        VncCredentialPreferences::new(Some(configuration)).upcast()
    }

    fn store_credentials(
        &self,
        preferences: &gtk::Widget,
        configuration: &mut ConnectionConfiguration,
    ) -> anyhow::Result<()> {
        let preferences = preferences
            .downcast_ref::<VncCredentialPreferences>()
            .expect("store_credentials got invalid widget type");
        todo!()
    }

    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> anyhow::Result<Box<dyn Connection>> {
        todo!()
    }
}

pub struct VncConnection;

impl Connection for VncConnection {
    fn metadata(&self) -> &ConnectionMetadata {
        todo!()
    }

    fn servers(&self) -> &[&dyn ServerConnection] {
        todo!()
    }
}

pub struct VncServerConnection;

impl ServerConnection for VncServerConnection {
    fn metadata(&self) -> &ServerMetadata {
        todo!()
    }

    fn adapters(&self) -> &[&dyn Adapter] {
        todo!()
    }
}
