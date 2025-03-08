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
use crate::api::connection::ProxmoxConnection;
use crate::credential_preferences::ProxmoxCredentialPreferences;
use crate::preferences::{ProxmoxConfiguration, ProxmoxPreferences};
use crate::tokiort::run_on_tokio;
use adw::prelude::Cast;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use gtk::Widget;
use libfieldmonitor::connection::{
    Connection, ConnectionConfiguration, ConnectionProvider, ConnectionProviderConstructor,
    ConnectionResult, DualScopedConnectionConfiguration, IconSpec, PreferencesGroupOrPage,
};
use std::borrow::Cow;

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
        gettext("Proxmox hypervisor connection").into()
    }

    fn icon(&self) -> IconSpec<()> {
        IconSpec::Named("connection-proxmox-symbolic".into())
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
