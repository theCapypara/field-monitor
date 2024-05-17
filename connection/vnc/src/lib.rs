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
use std::num::NonZeroU32;
use std::rc::Rc;

use adw::prelude::*;
use anyhow::anyhow;
use futures::future::LocalBoxFuture;
use futures::lock::Mutex;
use gettextrs::gettext;

use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::connection::*;

use crate::credential_preferences::VncCredentialPreferences;
use crate::preferences::{VncConfiguration, VncPreferences};

mod credential_preferences;
mod preferences;
mod util;

pub struct VncConnectionProviderConstructor;

impl ConnectionProviderConstructor for VncConnectionProviderConstructor {
    fn new(&self) -> Box<dyn ConnectionProvider> {
        Box::new(VncConnectionProvider {})
    }
}

pub struct VncConnectionProvider {}

impl ConnectionProvider for VncConnectionProvider {
    fn tag(&self) -> &'static str {
        "vnc"
    }

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

    fn preferences(
        &self,
        configuration: Option<Rc<Mutex<ConnectionConfiguration>>>,
    ) -> gtk::Widget {
        VncPreferences::new(configuration).upcast()
    }

    fn update_connection(
        &self,
        preferences: gtk::Widget,
        configuration: Rc<Mutex<ConnectionConfiguration>>,
    ) -> LocalBoxFuture<anyhow::Result<()>> {
        Box::pin(async {
            let preferences = preferences
                .downcast::<VncPreferences>()
                .expect("update_connection got invalid widget type");

            // Update general config
            {
                let mut config_lock = configuration.lock().await;
                config_lock.set_title(&preferences.title());
                config_lock.set_host(&preferences.host());
                let port_str = preferences.port();
                let Ok(port_int) = port_str.parse::<u32>() else {
                    preferences.port_entry_error(true);
                    return Err(anyhow!(gettext("Please enter a valid port")));
                };
                let Some(port_nzint) = NonZeroU32::new(port_int) else {
                    preferences.port_entry_error(true);
                    return Err(anyhow!(gettext("Please enter a valid port")));
                };
                config_lock.set_port(port_nzint);
            }

            // Update credentials
            let credentials = preferences.credentials();
            self.store_credentials(credentials.clone().upcast(), configuration)
                .await
        })
    }

    fn configure_credentials(
        &self,
        configuration: Rc<Mutex<ConnectionConfiguration>>,
    ) -> gtk::Widget {
        VncCredentialPreferences::new(Some(configuration)).upcast()
    }

    fn store_credentials(
        &self,
        preferences: gtk::Widget,
        configuration: Rc<Mutex<ConnectionConfiguration>>,
    ) -> LocalBoxFuture<anyhow::Result<()>> {
        Box::pin(async move {
            let preferences = preferences
                .downcast::<VncCredentialPreferences>()
                .expect("store_credentials got invalid widget type");

            let mut config_lock = configuration.lock().await;
            config_lock.set_user(preferences.user_if_remembered().as_deref());
            config_lock.set_password(preferences.password_if_remembered().as_deref());
            Ok(())
        })
    }

    fn load_connection(
        &self,
        configuration: &ConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<Box<dyn Connection>>> {
        todo!()
    }
}

pub struct VncConnection;

impl Connection for VncConnection {
    fn metadata(&self) -> &ConnectionMetadata {
        todo!()
    }

    fn servers(&self) -> anyhow::Result<&[&dyn ServerConnection]> {
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
