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

use adw::prelude::*;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;

use libfieldmonitor::connection::*;

use crate::connection::LibvirtConnection;
use crate::qemu_preferences::{LibvirtQemuConfiguration, LibvirtQemuPreferences, SessionType};

pub struct LibvirtQemuConnectionProviderConstructor;

impl ConnectionProviderConstructor for LibvirtQemuConnectionProviderConstructor {
    fn new(&self) -> Box<dyn ConnectionProvider> {
        Box::new(LibvirtQemuConnectionProvider {})
    }
}

pub struct LibvirtQemuConnectionProvider {}

impl ConnectionProvider for LibvirtQemuConnectionProvider {
    fn tag(&self) -> &'static str {
        "libvirt-qemu"
    }

    fn title(&self) -> Cow<'static, str> {
        gettext("QEMU/KVM").into()
    }

    fn title_plural(&self) -> Cow<str> {
        gettext("QEMU/KVM").into()
    }

    fn add_title(&self) -> Cow<str> {
        gettext("Add QEMU/KVM Connection").into()
    }

    fn title_for<'a>(&self, config: &'a ConnectionConfiguration) -> Option<&'a str> {
        config.title()
    }

    fn description(&self) -> Cow<str> {
        gettext("Setup a QEMU/KVM hypervisor connection via libvirt.").into()
    }

    fn preferences(&self, configuration: Option<&ConnectionConfiguration>) -> gtk::Widget {
        LibvirtQemuPreferences::new(configuration).upcast()
    }

    fn update_connection(
        &self,
        preferences: gtk::Widget,
        configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        self.store_credentials(&[], preferences, configuration)
    }

    fn configure_credentials(
        &self,
        _server_path: &[String],
        configuration: &ConnectionConfiguration,
    ) -> PreferencesGroupOrPage {
        PreferencesGroupOrPage::Page(LibvirtQemuPreferences::new(Some(configuration)).upcast())
    }

    fn store_credentials(
        &self,
        _server_path: &[String],
        preferences: gtk::Widget,
        mut configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async move {
            let preferences = preferences
                .downcast::<LibvirtQemuPreferences>()
                .expect("store_credentials got invalid widget type");

            configuration = configuration.transform_update_unified(|config| {
                config.set_title(&preferences.title());
                config.set_user_session(preferences.session_type() == SessionType::User);
                config.set_use_ssh(preferences.use_ssh());
                config.set_ssh_hostname(&preferences.ssh_hostname());
                config.set_ssh_username(&preferences.ssh_username());
                Result::<(), Infallible>::Ok(())
            })?;
            Ok(configuration)
        })
    }

    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<ConnectionResult<Box<dyn Connection>>> {
        let hostname: Cow<str> = if configuration.use_ssh() {
            configuration.ssh_hostname().to_string().into()
        } else {
            "localhost".into()
        };
        Box::pin(async move {
            let conn: Box<dyn Connection> = Box::new(
                LibvirtConnection::new(
                    &hostname,
                    &Self::build_uri(&configuration),
                    configuration.title().unwrap_or_default(),
                )
                .await?,
            );
            Ok(conn)
        })
    }
}

const SSH_OPTS: &str = "?no_tty=1";

impl LibvirtQemuConnectionProvider {
    fn build_uri(configuration: &ConnectionConfiguration) -> String {
        let session_type = if configuration.user_session() {
            "session"
        } else {
            "system"
        };

        let (suffix, ssh_part, params) = if configuration.use_ssh() {
            if configuration.ssh_username().is_empty() {
                (
                    "+ssh",
                    format!("{}/", configuration.ssh_hostname()),
                    SSH_OPTS,
                )
            } else {
                (
                    "+ssh",
                    format!(
                        "{}@{}/",
                        configuration.ssh_username(),
                        configuration.ssh_hostname()
                    ),
                    SSH_OPTS,
                )
            }
        } else {
            ("", "/".into(), "")
        };

        format!("qemu{suffix}://{ssh_part}{session_type}{params}")
    }
}
