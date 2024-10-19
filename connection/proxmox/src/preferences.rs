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
use std::cell::Cell;
use std::cell::RefCell;
use std::num::NonZeroU32;

use adw::subclass::prelude::*;
use anyhow::anyhow;
use futures::future::BoxFuture;
use glib::clone;
use gtk::glib;
use gtk::prelude::*;
use secure_string::SecureString;

use libfieldmonitor::connection::{ConfigAccess, ConfigAccessMut, ConnectionConfiguration};

use crate::credential_preferences::ProxmoxCredentialPreferences;

pub(super) trait ProxmoxConfiguration {
    fn title(&self) -> Option<&str>;
    fn set_title(&mut self, value: &str);
    fn ignore_ssl_cert_error(&self) -> bool;
    fn set_ignore_ssl_cert_error(&mut self, value: bool);
    fn hostname(&self) -> Option<&str>;
    fn set_hostname(&mut self, value: &str);
    fn port(&self) -> Option<NonZeroU32>;
    fn set_port(&mut self, value: NonZeroU32);
    fn use_apikey(&self) -> bool;
    fn set_use_apikey(&mut self, value: bool);
    fn username(&self) -> Option<&str>;
    fn set_username(&mut self, value: &str);
    fn tokenid(&self) -> Option<&str>;
    fn set_tokenid(&mut self, value: &str);
    fn password_or_apikey(&self) -> BoxFuture<anyhow::Result<Option<SecureString>>>;
    fn set_password_or_apikey(&mut self, value: Option<SecureString>);
    fn set_password_or_apikey_session(&mut self, value: Option<SecureString>);
}

impl ProxmoxConfiguration for ConnectionConfiguration {
    fn title(&self) -> Option<&str> {
        self.get_try_as_str("title")
    }

    fn set_title(&mut self, value: &str) {
        self.set_value("title", value);
    }

    fn ignore_ssl_cert_error(&self) -> bool {
        self.get_try_as_bool("ignore-ssl-cert-error")
            .unwrap_or_default()
    }

    fn set_ignore_ssl_cert_error(&mut self, value: bool) {
        self.set_value("ignore-ssl-cert-error", value);
    }

    fn hostname(&self) -> Option<&str> {
        self.get_try_as_str("hostname")
    }

    fn set_hostname(&mut self, value: &str) {
        self.set_value("hostname", value);
    }

    fn port(&self) -> Option<NonZeroU32> {
        self.get_try_as_u64("port").and_then(|v| {
            if v <= (u32::MAX as u64) {
                NonZeroU32::new(v as u32)
            } else {
                None
            }
        })
    }

    fn set_port(&mut self, value: NonZeroU32) {
        self.set_value("port", value.get());
    }

    fn use_apikey(&self) -> bool {
        self.get_try_as_bool("use-apikey").unwrap_or_default()
    }

    fn set_use_apikey(&mut self, value: bool) {
        self.set_value("use-apikey", value);
    }

    fn username(&self) -> Option<&str> {
        self.get_try_as_str("username")
    }

    fn set_username(&mut self, value: &str) {
        self.set_value("username", value);
    }

    fn tokenid(&self) -> Option<&str> {
        self.get_try_as_str("tokenid")
    }

    fn set_tokenid(&mut self, value: &str) {
        self.set_value("tokenid", value);
    }

    fn password_or_apikey(&self) -> BoxFuture<anyhow::Result<Option<SecureString>>> {
        Box::pin(async move {
            if let Some(pw) = self.get_try_as_sec_string("__session__password-or-apikey") {
                return Ok(Some(pw));
            }
            self.get_secret("password-or-apikey").await
        })
    }

    fn set_password_or_apikey(&mut self, value: Option<SecureString>) {
        match value {
            None => self.clear_secret("password-or-apikey"),
            Some(value) => self.set_secret("password-or-apikey", value),
        }
    }

    fn set_password_or_apikey_session(&mut self, value: Option<SecureString>) {
        match value {
            None => {
                self.clear("__session__password-or-apikey");
            }
            Some(value) => {
                self.set_secure_string("__session__password-or-apikey", value.clone());
            }
        }
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::ProxmoxPreferences)]
    #[template(resource = "/de/capypara/FieldMonitor/connection/proxmox/preferences.ui")]
    pub struct ProxmoxPreferences {
        #[template_child]
        pub credentials: TemplateChild<ProxmoxCredentialPreferences>,
        #[template_child]
        pub port_entry: TemplateChild<adw::EntryRow>,
        #[property(get, set)]
        title: RefCell<String>,
        #[property(get, set)]
        hostname: RefCell<String>,
        #[property(get, set)]
        port: RefCell<String>,
        #[property(get, set)]
        ignore_ssl_cert_error: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProxmoxPreferences {
        const NAME: &'static str = "ProxmoxPreferences";
        type Type = super::ProxmoxPreferences;
        type ParentType = adw::PreferencesPage;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProxmoxPreferences {}
    impl WidgetImpl for ProxmoxPreferences {}
    impl PreferencesPageImpl for ProxmoxPreferences {}
}

glib::wrapper! {
    pub struct ProxmoxPreferences(ObjectSubclass<imp::ProxmoxPreferences>)
        @extends gtk::Widget, adw::PreferencesPage;
}

impl ProxmoxPreferences {
    pub fn new(existing_configuration: Option<&ConnectionConfiguration>) -> Self {
        let slf: Self = glib::Object::builder().build();

        if let Some(existing_configuration) = existing_configuration.cloned() {
            glib::spawn_future_local(clone!(
                #[weak]
                slf,
                async move {
                    slf.set_title(existing_configuration.title().unwrap_or_default());
                    slf.set_hostname(existing_configuration.hostname().unwrap_or_default());
                    slf.set_port(
                        existing_configuration
                            .port()
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                    );
                    slf.set_ignore_ssl_cert_error(existing_configuration.ignore_ssl_cert_error());

                    slf.imp()
                        .credentials
                        .propagate_settings(&existing_configuration)
                        .await;
                }
            ));
        }

        slf
    }

    pub fn apply_general_config(
        &self,
        config: &mut ConnectionConfiguration,
    ) -> Result<(), anyhow::Error> {
        let Some(port) = self
            .port()
            .parse::<u32>()
            .ok()
            .and_then(|v| NonZeroU32::try_from(v).ok())
        else {
            self.port_entry_error(true);
            return Err(anyhow!("invalid port"));
        };
        self.port_entry_error(false);

        config.set_title(&self.title());
        config.set_hostname(&self.hostname());
        config.set_port(port);
        config.set_ignore_ssl_cert_error(self.ignore_ssl_cert_error());

        Ok(())
    }

    pub fn port_entry_error(&self, error: bool) {
        if error {
            self.imp().port_entry.add_css_class("error");
        } else {
            self.imp().port_entry.remove_css_class("error");
        }
    }

    pub fn credentials(&self) -> &ProxmoxCredentialPreferences {
        &self.imp().credentials
    }
}

#[gtk::template_callbacks]
impl ProxmoxPreferences {}
