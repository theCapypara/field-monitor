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
use std::str::FromStr;
use std::sync::Arc;

use adw::prelude::Cast;
use anyhow::anyhow;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use gtk::Widget;
use http::Uri;
use secure_string::SecureString;

use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::connection::{
    Actionable, Connection, ConnectionConfiguration, ConnectionError, ConnectionMetadata,
    ConnectionProvider, ConnectionProviderConstructor, ConnectionResult,
    DualScopedConnectionConfiguration, PreferencesGroupOrPage, ServerConnection, ServerMap,
    ServerMetadata,
};
use proxmox_api::ProxmoxApiClient;

use crate::credential_preferences::ProxmoxCredentialPreferences;
use crate::preferences::{ProxmoxConfiguration, ProxmoxPreferences};
use crate::tokiort::run_on_tokio;

mod credential_preferences;
mod preferences;
mod tokiort;

pub struct ProxmoxConnectionProviderConstructor;

impl ConnectionProviderConstructor for ProxmoxConnectionProviderConstructor {
    fn new(&self) -> Box<dyn ConnectionProvider> {
        Box::new(ProxmoxConnectionProvider {})
    }
}

pub struct ProxmoxConnectionProvider {}

impl ConnectionProvider for ProxmoxConnectionProvider {
    fn tag(&self) -> &'static str {
        "proxmox"
    }

    fn title(&self) -> Cow<'static, str> {
        gettext("Proxmox").into()
    }

    fn title_plural(&self) -> Cow<str> {
        gettext("Proxmox").into()
    }

    fn add_title(&self) -> Cow<str> {
        gettext("Add Proxmox Connection").into()
    }

    fn title_for<'a>(&self, config: &'a ConnectionConfiguration) -> Option<&'a str> {
        config.title()
    }

    fn description(&self) -> Cow<str> {
        gettext("Setup a Proxmox hypervisor connection.").into()
    }

    fn preferences(&self, configuration: Option<&ConnectionConfiguration>) -> Widget {
        ProxmoxPreferences::new(configuration).upcast()
    }

    fn update_connection(
        &self,
        preferences: Widget,
        mut configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async {
            let preferences = preferences
                .downcast::<ProxmoxPreferences>()
                .expect("update_connection got invalid widget type");

            // Update general config
            configuration = configuration
                .transform_update_unified(|config| preferences.apply_general_config(config))?;

            // Update credentials
            let credentials = preferences.credentials();
            self.store_credentials(&[], credentials.clone().upcast(), configuration)
                .await
        })
    }

    fn configure_credentials(
        &self,
        _server_path: &[String],
        configuration: &ConnectionConfiguration,
    ) -> PreferencesGroupOrPage {
        PreferencesGroupOrPage::Group(
            ProxmoxCredentialPreferences::new(Some(configuration), true).upcast(),
        )
    }

    fn store_credentials(
        &self,
        _server_path: &[String],
        preferences: Widget,
        configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async move {
            let preferences = preferences
                .downcast::<ProxmoxCredentialPreferences>()
                .expect("store_credentials got invalid widget type");

            configuration.transform_update_separate(
                |c_session| preferences.apply_persistent_config(c_session),
                |c_persistent| preferences.apply_session_config(c_persistent),
            )
        })
    }

    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<ConnectionResult<Box<dyn Connection>>> {
        Box::pin(async move {
            let con: ProxmoxConnection =
                run_on_tokio(ProxmoxConnection::connect(configuration)).await?;
            let conbx: Box<dyn Connection> = Box::new(con);
            Ok(conbx)
        })
    }
}

struct ProxmoxConnection(Arc<ProxmoxApiClient>);

impl ProxmoxConnection {
    async fn connect(config: ConnectionConfiguration) -> ConnectionResult<Self> {
        let authority = format!(
            "{}:{}",
            config.hostname().unwrap_or_default(),
            config.port().map(NonZeroU32::get).unwrap_or(8006)
        );

        let api_root = Uri::builder()
            .scheme("https")
            .authority(authority)
            .path_and_query("/api2/json")
            .build()
            .map_err(|err| {
                ConnectionError::General(
                    Some(gettext(
                        "Was unable to build a valid URL to connect to. Check your settings.",
                    )),
                    anyhow!(err),
                )
            })?;

        let pass = config
            .password_or_apikey()
            .await
            .map_err(|err| {
                ConnectionError::General(
                    Some(gettext(
                        "Failed to retrieve API Key or Password from secrets service.",
                    )),
                    anyhow!(err),
                )
            })?
            .unwrap_or_else(|| SecureString::from_str("").unwrap());

        let client = if config.use_apikey() {
            ProxmoxApiClient::connect_with_apikey(
                &api_root,
                config.tokenid().unwrap_or_default(),
                pass,
                config.ignore_ssl_cert_error(),
            )
            .await
            .map_err(map_proxmox_error)
        } else {
            ProxmoxApiClient::connect_with_ticket(
                &api_root,
                config.username().unwrap_or_default(),
                pass,
                config.ignore_ssl_cert_error(),
            )
            .await
            .map_err(map_proxmox_error)
        }?;

        Ok(Self(Arc::new(client)))
    }
}

impl Actionable for ProxmoxConnection {}

impl Connection for ProxmoxConnection {
    fn metadata(&self) -> ConnectionMetadata {
        todo!()
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        todo!()
    }
}

struct ProxmoxServer;

impl Actionable for ProxmoxServer {}

impl ServerConnection for ProxmoxServer {
    fn metadata(&self) -> ServerMetadata {
        todo!()
    }

    fn supported_adapters(&self) -> Vec<(Cow<str>, Cow<str>)> {
        todo!()
    }

    fn create_adapter(&self, tag: &str) -> LocalBoxFuture<ConnectionResult<Box<dyn Adapter>>> {
        todo!()
    }
}

struct ProxmoxVm;

impl Actionable for ProxmoxVm {}

impl ServerConnection for ProxmoxVm {
    fn metadata(&self) -> ServerMetadata {
        todo!()
    }

    fn supported_adapters(&self) -> Vec<(Cow<str>, Cow<str>)> {
        todo!()
    }

    fn create_adapter(&self, tag: &str) -> LocalBoxFuture<ConnectionResult<Box<dyn Adapter>>> {
        todo!()
    }
}

fn map_proxmox_error(error: proxmox_api::Error) -> ConnectionError {
    todo!()
}
