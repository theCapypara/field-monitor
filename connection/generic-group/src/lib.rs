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
mod credential_preferences;
mod preferences;
mod server_config;
mod server_preferences;
mod util;

use adw::prelude::*;
use std::borrow::Cow;
use std::num::NonZeroU32;
use std::rc::Rc;

use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use indexmap::IndexMap;
use secure_string::SecureString;

use crate::credential_preferences::GenericGroupCredentialPreferences;
use crate::preferences::{GenericGroupConfiguration, GenericGroupPreferences};
use crate::server_config::FinalizedServerConfig;
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::config_error;
use libfieldmonitor::connection::*;

pub struct GenericConnectionProviderConstructor;

impl ConnectionProviderConstructor for GenericConnectionProviderConstructor {
    fn new(&self) -> Box<dyn ConnectionProvider> {
        Box::new(GenericConnectionProvider {})
    }
}

pub struct GenericConnectionProvider {}

impl ConnectionProvider for GenericConnectionProvider {
    fn tag(&self) -> &'static str {
        "generic"
    }

    fn title(&self) -> Cow<'static, str> {
        gettext("Generic Connection Group").into()
    }

    fn title_plural(&self) -> Cow<str> {
        gettext("Generic Connection Groups").into()
    }

    fn add_title(&self) -> Cow<str> {
        gettext("Add Generic Connection Group").into()
    }

    fn title_for<'a>(&self, config: &'a ConnectionConfiguration) -> Option<&'a str> {
        config.connection_title()
    }

    fn description(&self) -> Cow<str> {
        gettext("Connection to one or more RDP, SPICE and VNC servers").into()
    }

    fn icon(&self) -> IconSpec<()> {
        IconSpec::Default
    }

    fn preferences(&self, configuration: Option<&ConnectionConfiguration>) -> gtk::Widget {
        GenericGroupPreferences::new(configuration).upcast()
    }

    fn update_connection(
        &self,
        preferences: gtk::Widget,
        configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async {
            let preferences = preferences
                .downcast::<GenericGroupPreferences>()
                .expect("update_connection got invalid widget type");

            let server_changes = &*preferences.servers();

            configuration.transform_update_separate(
                |c_session| {
                    c_session.set_connection_title(&preferences.title());

                    for server in server_changes.updates.values() {
                        c_session.set_server_type(&server.key, server.server_type);
                        c_session.set_title(&server.key, &server.title);
                        c_session.set_host(&server.key, &server.host);
                        c_session.set_port(&server.key, server.port);
                        store_credentials_session(&server.key, server, c_session)?
                    }

                    for removal in &server_changes.removes {
                        c_session.remove_server(removal);
                    }
                    Ok(())
                },
                |c_persistent| {
                    c_persistent.set_connection_title(&preferences.title());

                    for server in server_changes.updates.values() {
                        c_persistent.set_server_type(&server.key, server.server_type);
                        c_persistent.set_title(&server.key, &server.title);
                        c_persistent.set_host(&server.key, &server.host);
                        c_persistent.set_port(&server.key, server.port);
                        store_credentials_persistent(&server.key, server, c_persistent)?
                    }

                    for removal in &server_changes.removes {
                        c_persistent.remove_server(removal);
                    }
                    Ok(())
                },
            )
        })
    }

    fn configure_credentials(
        &self,
        server: &[String],
        configuration: &ConnectionConfiguration,
    ) -> PreferencesGroupOrPage {
        PreferencesGroupOrPage::Group(
            GenericGroupCredentialPreferences::new(&server.join("/"), Some(configuration), true)
                .upcast(),
        )
    }

    fn store_credentials(
        &self,
        server: &[String],
        preferences: gtk::Widget,
        configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        let server = server.to_vec();
        Box::pin(async move {
            let server = server.join("/");
            let preferences = preferences
                .downcast::<GenericGroupCredentialPreferences>()
                .expect("store_credentials got invalid widget type");

            configuration.transform_update_separate(
                |c_session| {
                    store_credentials_session(
                        &server,
                        &preferences.as_incomplete_server_config(),
                        c_session,
                    )
                },
                |c_persistent| {
                    store_credentials_persistent(
                        &server,
                        &preferences.as_incomplete_server_config(),
                        c_persistent,
                    )
                },
            )
        })
    }

    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<ConnectionResult<Box<dyn Connection>>> {
        Box::pin(async move {
            let title = configuration
                .connection_title()
                .ok_or_else(|| config_error(None))?
                .to_string();

            let c: Box<dyn Connection> = Box::new(GenericConnection::new(title, configuration));
            Ok(c)
        })
    }
}

fn store_credentials_session(
    server: &str,
    preferences: &FinalizedServerConfig,
    c_session: &mut ConnectionConfiguration,
) -> anyhow::Result<()> {
    c_session.set_user(server, preferences.user.as_deref());
    c_session.set_password_session(server, preferences.password.as_ref());
    Ok(())
}

fn store_credentials_persistent(
    server: &str,
    preferences: &FinalizedServerConfig,
    c_persistent: &mut ConnectionConfiguration,
) -> anyhow::Result<()> {
    c_persistent.set_user(server, preferences.user_if_remembered());
    c_persistent.set_password(server, preferences.password_if_remembered().cloned());

    Ok(())
}

#[derive(Clone)]
pub struct GenericConnection {
    title: String,
    config: Rc<ConnectionConfiguration>,
}

impl Actionable for GenericConnection {}

impl Connection for GenericConnection {
    fn metadata(&self) -> LocalBoxFuture<ConnectionMetadata> {
        Box::pin(async {
            ConnectionMetadataBuilder::default()
                .title(self.title.clone())
                .build()
                .unwrap()
        })
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            let mut hm: IndexMap<_, Box<dyn ServerConnection>> = IndexMap::with_capacity(1);

            let mut keys = self.config.section_keys().collect::<Vec<_>>();
            keys.sort_by_key(|key| self.config.title(key).unwrap_or_default());

            for server in keys {
                hm.insert(
                    server.to_string().into(),
                    Box::new(GenericConnectionServer {
                        key: server.to_string(),
                        config: self.config.clone(),
                    }),
                );
            }

            Ok(hm)
        })
    }
}

impl GenericConnection {
    fn new(title: String, config: ConnectionConfiguration) -> Self {
        Self {
            title,
            config: Rc::new(config),
        }
    }
}

struct GenericConnectionServer {
    key: String,
    config: Rc<ConnectionConfiguration>,
}

impl Actionable for GenericConnectionServer {}

impl ServerConnection for GenericConnectionServer {
    fn metadata(&self) -> LocalBoxFuture<ServerMetadata> {
        Box::pin(async {
            let user_part = self
                .config
                .user(&self.key)
                .map(|u| format!("{u}@"))
                .unwrap_or_default();
            ServerMetadataBuilder::default()
                .title(self.config.title(&self.key).unwrap_or_default())
                .subtitle(Some(format!(
                    "{}://{}{}:{}",
                    self.config
                        .server_type(&self.key)
                        .map(|s| s.protocol())
                        .unwrap_or_default(),
                    user_part,
                    self.config.host(&self.key).unwrap_or_default(),
                    self.config
                        .port(&self.key)
                        .map(u32::from)
                        .unwrap_or_default()
                )))
                .build()
                .unwrap()
        })
    }

    fn supported_adapters(&self) -> LocalBoxFuture<Vec<(Cow<str>, Cow<str>)>> {
        Box::pin(async {
            let server_type = self.config.server_type(&self.key);
            if let Some(server_type) = server_type {
                vec![(server_type.tag().into(), server_type.label())]
            } else {
                vec![]
            }
        })
    }

    fn create_adapter(
        &self,
        tag: &str,
    ) -> LocalBoxFuture<Result<Box<dyn Adapter>, ConnectionError>> {
        let server_type = self.config.server_type(&self.key);
        assert_eq!(
            tag,
            server_type.map(|s| s.tag()).unwrap_or_default(),
            "unsupported adapter type"
        );
        let server_type = server_type.unwrap();

        Box::pin(async move {
            let password = match self.config.password(&self.key).await {
                Ok(pass) => pass.unwrap_or_else(|| SecureString::from("")),
                Err(err) => {
                    return Err(ConnectionError::AuthFailed(
                        Some(gettext("Failed to load password.")),
                        err,
                    ));
                }
            };

            let bx = server_type.new_adapter(
                self.config
                    .host(&self.key)
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                self.config
                    .port(&self.key)
                    .as_ref()
                    .copied()
                    .map(NonZeroU32::get)
                    .unwrap_or_default(),
                self.config
                    .user(&self.key)
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                password,
            );

            Ok(bx)
        })
    }
}
