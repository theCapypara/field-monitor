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
use std::convert::Infallible;
use std::num::NonZeroU32;

use adw::prelude::*;
use anyhow::anyhow;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use indexmap::IndexMap;
use secure_string::SecureString;

use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::config_error;
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

    fn title(&self) -> Cow<'static, str> {
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

    fn update_connection(
        &self,
        preferences: gtk::Widget,
        mut configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async {
            let preferences = preferences
                .downcast::<VncPreferences>()
                .expect("update_connection got invalid widget type");

            configuration = configuration.transform_update_unified(|configuration| {
                // Update general config
                configuration.set_title(&preferences.title());
                configuration.set_host(&preferences.host());
                let port_str = preferences.port();
                let Ok(port_int) = port_str.parse::<u32>() else {
                    preferences.port_entry_error(true);
                    return Err(anyhow!(gettext("Please enter a valid port")));
                };
                let Some(port_nzint) = NonZeroU32::new(port_int) else {
                    preferences.port_entry_error(true);
                    return Err(anyhow!(gettext("Please enter a valid port")));
                };
                configuration.set_port(port_nzint);
                Ok(())
            })?;

            // Update credentials
            let credentials = preferences.credentials();
            self.store_credentials(credentials.clone().upcast(), configuration)
                .await
        })
    }

    fn configure_credentials(
        &self,
        configuration: &ConnectionConfiguration,
    ) -> adw::PreferencesGroup {
        VncCredentialPreferences::new(Some(configuration), true).upcast()
    }

    fn store_credentials(
        &self,
        preferences: adw::PreferencesGroup,
        mut configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async move {
            let preferences = preferences
                .downcast::<VncCredentialPreferences>()
                .expect("store_credentials got invalid widget type");

            configuration = configuration.transform_update_separate(
                |c_session| {
                    c_session.set_user(Some(&preferences.user()));
                    c_session
                        .set_password_session(Some(&SecureString::from(preferences.password())));
                    Result::<(), Infallible>::Ok(())
                },
                |c_persistent| {
                    c_persistent.set_user(preferences.user_if_remembered().as_deref());
                    c_persistent
                        .set_password(preferences.password_if_remembered().map(SecureString::from));
                    Result::<(), Infallible>::Ok(())
                },
            )?;

            Ok(configuration)
        })
    }

    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<ConnectionResult<Box<dyn Connection>>> {
        Box::pin(async move {
            let title = configuration
                .title()
                .ok_or_else(|| config_error(None))?
                .to_string();

            let c: Box<dyn Connection> = Box::new(VncConnection::new(title, configuration));
            Ok(c)
        })
    }
}

#[derive(Clone)]
pub struct VncConnection {
    title: String,
    config: ConnectionConfiguration,
}

impl Actionable for VncConnection {}

impl Connection for VncConnection {
    fn metadata(&self) -> ConnectionMetadata {
        ConnectionMetadataBuilder::default()
            .title(self.title.clone())
            .build()
            .unwrap()
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            let mut hm: IndexMap<_, Box<dyn ServerConnection>> = IndexMap::with_capacity(1);

            hm.insert(Cow::Borrowed("server"), Box::new(self.clone()));

            Ok(hm)
        })
    }
}

impl VncConnection {
    fn new(title: String, config: ConnectionConfiguration) -> Self {
        Self { title, config }
    }
}

impl ServerConnection for VncConnection {
    fn metadata(&self) -> ServerMetadata {
        ServerMetadataBuilder::default()
            .title(self.title.clone())
            .build()
            .unwrap()
    }

    fn supported_adapters(&self) -> Vec<(Cow<str>, Cow<str>)> {
        vec![(VncAdapter::TAG, VncAdapter::label())]
    }

    fn create_adapter(
        &self,
        tag: &str,
    ) -> LocalBoxFuture<Result<Box<dyn Adapter>, ConnectionError>> {
        assert_eq!(tag, VncAdapter::TAG, "unsupported adapter type");
        Box::pin(async move {
            let password = match self.config.password().await {
                Ok(pass) => pass.unwrap_or_else(|| SecureString::from("")),
                Err(err) => {
                    return Err(ConnectionError::AuthFailed(
                        Some(gettext("Failed to load password.")),
                        err,
                    ));
                }
            };

            let bx: Box<dyn Adapter> = Box::new(VncAdapter::new(
                self.config
                    .host()
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                self.config
                    .port()
                    .as_ref()
                    .copied()
                    .map(NonZeroU32::get)
                    .unwrap_or_default(),
                self.config
                    .user()
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                password,
            ));

            Ok(bx)
        })
    }
}
