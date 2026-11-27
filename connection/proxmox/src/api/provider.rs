/* Copyright 2024-2026 Marco Köpcke
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
use crate::api::connection::{ClientCache, ProxmoxConnection};
use crate::credential_preferences::ProxmoxCredentialPreferences;
use crate::preferences::{ProxmoxConfiguration, ProxmoxPreferences};
use adw::prelude::Cast;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use gtk::Widget;
use libfieldmonitor::connection::{
    Connection, ConnectionConfiguration, ConnectionProvider, ConnectionProviderConstructor,
    ConnectionResult, DualScopedConnectionConfiguration, IconSpec, PreferencesGroupOrPage,
};
use libfieldmonitor::tokiort::run_on_tokio;
use std::borrow::Cow;

pub struct ProxmoxConnectionProviderConstructor;

impl ConnectionProviderConstructor for ProxmoxConnectionProviderConstructor {
    fn new(&self) -> Box<dyn ConnectionProvider> {
        Box::new(ProxmoxConnectionProvider::default())
    }
}

#[derive(Default)]
pub struct ProxmoxConnectionProvider {
    clients: ClientCache,
}

impl ConnectionProvider for ProxmoxConnectionProvider {
    fn tag(&self) -> &'static str {
        "proxmox"
    }

    fn title(&self) -> Cow<'static, str> {
        gettext("Proxmox").into()
    }

    fn title_plural(&self) -> Cow<'_, str> {
        gettext("Proxmox").into()
    }

    fn add_title(&self) -> Cow<'_, str> {
        gettext("Add Proxmox Connection").into()
    }

    fn title_for<'a>(&self, config: &'a ConnectionConfiguration) -> Option<&'a str> {
        config.title()
    }

    fn description(&self) -> Cow<'_, str> {
        gettext("Proxmox Virtual Environment hypervisor connection").into()
    }

    fn icon(&self) -> IconSpec<()> {
        IconSpec::Named("connection-proxmox-symbolic".into())
    }

    fn preferences(
        &self,
        configuration: Option<&ConnectionConfiguration>,
        _server_path: Option<&[String]>,
    ) -> Widget {
        ProxmoxPreferences::new(configuration).upcast()
    }

    fn update_connection(
        &self,
        preferences: Widget,
        mut configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<'_, anyhow::Result<DualScopedConnectionConfiguration>> {
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
    ) -> LocalBoxFuture<'_, anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async move {
            let preferences = preferences
                .downcast::<ProxmoxCredentialPreferences>()
                .expect("store_credentials got invalid widget type");

            let configuration = configuration.transform_update_separate(
                |c_session| preferences.apply_session_config(c_session),
                |c_persistent| preferences.apply_persistent_config(c_persistent),
            )?;
            if preferences.use_temporary_credentials() && !preferences.use_apikey() {
                let config = configuration.session().clone();
                let clients = self.clients.clone();
                let code = secure_string::SecureString::from(preferences.tfa_response().trim());
                preferences.set_tfa_response("");
                let result = run_on_tokio::<_, _, libfieldmonitor::connection::ConnectionError>(
                    async move {
                        let client = ProxmoxConnection::client_for(&config, clients).await?;
                        let result = if code.unsecure().is_empty() {
                            client.authenticate().await
                        } else {
                            client.submit_totp(code).await
                        };
                        result.map_err(crate::api::map_proxmox_error)
                    },
                )
                .await;
                if let Err(err) = result {
                    if matches!(
                        err.inner().downcast_ref::<proxmox_api::Error>(),
                        Some(proxmox_api::Error::TfaRequired | proxmox_api::Error::TfaRejected)
                    ) {
                        preferences.show_tfa_prompt();
                    }
                    return Err(anyhow::anyhow!("{}", err));
                }
            }
            Ok(configuration)
        })
    }

    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<'_, ConnectionResult<Box<dyn Connection>>> {
        Box::pin(async move {
            let con: ProxmoxConnection = run_on_tokio(ProxmoxConnection::connect(
                configuration,
                self.clients.clone(),
            ))
            .await?;
            let conbx: Box<dyn Connection> = Box::new(con);
            Ok(conbx)
        })
    }
}
