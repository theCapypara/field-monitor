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

use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use indexmap::IndexMap;
use secure_string::SecureString;

use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::config_error;
use libfieldmonitor::connection::*;

use crate::preferences::GenericGroupConfiguration;

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
        gettext("VNC Connection Group").into()
    }

    fn title_plural(&self) -> Cow<str> {
        gettext("VNC Connection Groups").into()
    }

    fn add_title(&self) -> Cow<str> {
        gettext("Add VNC Connection Group").into()
    }

    fn description(&self) -> Cow<str> {
        gettext("Setup a connection to one or more VNC servers.").into()
    }

    fn preferences(&self, configuration: Option<&ConnectionConfiguration>) -> gtk::Widget {
        super::preferences(configuration)
    }

    fn update_connection(
        &self,
        preferences: gtk::Widget,
        configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        super::update_connection(preferences, configuration)
    }

    fn configure_credentials(
        &self,
        server: &[String],
        configuration: &ConnectionConfiguration,
    ) -> adw::PreferencesGroup {
        super::configure_credentials(server, configuration)
    }

    fn store_credentials(
        &self,
        server: &[String],
        preferences: adw::PreferencesGroup,
        configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        let server = server.to_vec();
        Box::pin(async move { super::store_credentials(&server, preferences, configuration) })
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

            let c: Box<dyn Connection> = Box::new(VncConnection::new(title, configuration));
            Ok(c)
        })
    }
}

#[derive(Clone)]
pub struct VncConnection {
    title: String,
    config: Rc<ConnectionConfiguration>,
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

            let mut keys = self.config.section_keys().collect::<Vec<_>>();
            keys.sort_by_key(|key| self.config.title(key).unwrap_or_default());

            for server in keys {
                hm.insert(
                    server.to_string().into(),
                    Box::new(VncConnectionServer {
                        key: server.to_string(),
                        config: self.config.clone(),
                    }),
                );
            }

            Ok(hm)
        })
    }
}

impl VncConnection {
    fn new(title: String, config: ConnectionConfiguration) -> Self {
        Self {
            title,
            config: Rc::new(config),
        }
    }
}

struct VncConnectionServer {
    key: String,
    config: Rc<ConnectionConfiguration>,
}

impl Actionable for VncConnectionServer {}

impl ServerConnection for VncConnectionServer {
    fn metadata(&self) -> ServerMetadata {
        let user_part = self
            .config
            .user(&self.key)
            .map(|u| format!("{u}@"))
            .unwrap_or_default();
        ServerMetadataBuilder::default()
            .title(self.config.title(&self.key).unwrap_or_default())
            .subtitle(Some(format!(
                "{}{}:{}",
                user_part,
                self.config.host(&self.key).unwrap_or_default(),
                self.config
                    .port(&self.key)
                    .map(u32::from)
                    .unwrap_or_default()
            )))
            .build()
            .unwrap()
    }

    fn supported_adapters(&self) -> Vec<(Cow<str>, Cow<str>)> {
        vec![(VncAdapter::TAG.into(), VncAdapter::label())]
    }

    fn create_adapter(
        &self,
        tag: &str,
    ) -> LocalBoxFuture<Result<Box<dyn Adapter>, ConnectionError>> {
        assert_eq!(tag, VncAdapter::TAG, "unsupported adapter type");

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

            let bx: Box<dyn Adapter> = Box::new(VncAdapter::new(
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
            ));

            Ok(bx)
        })
    }
}