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
use futures::future::LocalBoxFuture;
use gtk::prelude::*;

use libfieldmonitor::connection::{
    ConnectionConfiguration, DualScopedConnectionConfiguration, PreferencesGroupOrPage,
};
pub use rdp::*;
pub use spice::*;
pub use vnc::*;

use crate::credential_preferences::GenericGroupCredentialPreferences;
use crate::preferences::{GenericGroupConfiguration, GenericGroupPreferences};
use crate::server_config::FinalizedServerConfig;

mod rdp;
mod spice;
mod vnc;

fn preferences(configuration: Option<&ConnectionConfiguration>) -> gtk::Widget {
    GenericGroupPreferences::new(configuration).upcast()
}

fn update_connection<'a>(
    preferences: gtk::Widget,
    configuration: DualScopedConnectionConfiguration,
) -> LocalBoxFuture<'a, anyhow::Result<DualScopedConnectionConfiguration>> {
    Box::pin(async {
        let preferences = preferences
            .downcast::<GenericGroupPreferences>()
            .expect("update_connection got invalid widget type");

        let server_changes = &*preferences.servers();

        configuration.transform_update_separate(
            |c_session| {
                c_session.set_connection_title(&preferences.title());

                for server in server_changes.updates.values() {
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
    server: &[String],
    configuration: &ConnectionConfiguration,
) -> PreferencesGroupOrPage {
    PreferencesGroupOrPage::Group(
        GenericGroupCredentialPreferences::new(&server.join("/"), Some(configuration), true)
            .upcast(),
    )
}

fn store_credentials(
    server: &[String],
    preferences: gtk::Widget,
    configuration: DualScopedConnectionConfiguration,
) -> anyhow::Result<DualScopedConnectionConfiguration> {
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
