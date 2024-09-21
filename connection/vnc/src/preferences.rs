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

use std::cell::RefCell;
use std::num::NonZeroU32;

use adw::glib::clone;
use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use secure_string::SecureString;

use libfieldmonitor::connection::ConnectionConfiguration;

use crate::credential_preferences::VncCredentialPreferences;

pub(super) trait VncConfiguration {
    fn title(&self) -> Option<&str>;
    fn host(&self) -> Option<&str>;
    fn port(&self) -> Option<NonZeroU32>;
    fn user(&self) -> Option<&str>;
    async fn password(&self) -> anyhow::Result<Option<SecureString>>;
    fn set_title(&mut self, value: &str);
    fn set_host(&mut self, value: &str);
    fn set_port(&mut self, value: NonZeroU32);
    fn set_user(&mut self, value: Option<&str>);
    fn set_password(&mut self, value: Option<SecureString>);
    fn set_password_session(&mut self, value: Option<&SecureString>);
}

impl VncConfiguration for ConnectionConfiguration {
    fn title(&self) -> Option<&str> {
        self.get_try_as_str("title")
    }

    fn host(&self) -> Option<&str> {
        self.get_try_as_str("host")
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

    fn user(&self) -> Option<&str> {
        self.get_try_as_str("user")
    }

    async fn password(&self) -> anyhow::Result<Option<SecureString>> {
        if let Some(pw) = self.get_try_as_sec_str("__session__password") {
            return Ok(Some(pw));
        }
        self.get_secret("password").await
    }

    fn set_title(&mut self, value: &str) {
        self.set_value("title", value);
    }

    fn set_host(&mut self, value: &str) {
        self.set_value("host", value);
    }

    fn set_port(&mut self, value: NonZeroU32) {
        self.set_value("port", value.get());
    }

    fn set_user(&mut self, value: Option<&str>) {
        let value = match value {
            None => serde_yaml::Value::Null,
            Some(value) => value.into(),
        };
        self.set_value("user", value);
    }

    fn set_password(&mut self, value: Option<SecureString>) {
        self.set_password_session(value.as_ref());
        match value {
            None => self.clear_secret("password"),
            Some(value) => self.set_secret("password", value),
        }
    }

    fn set_password_session(&mut self, value: Option<&SecureString>) {
        match value {
            None => {
                self.clear("__session__password");
            }
            Some(value) => {
                self.set_secure_string("__session__password", value.clone());
            }
        }
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::VncPreferences)]
    #[template(resource = "/de/capypara/FieldMonitor/connection/vnc/preferences.ui")]
    pub struct VncPreferences {
        #[template_child]
        pub(super) title_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) host_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) port_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) credentials: TemplateChild<VncCredentialPreferences>,

        #[property(get, set)]
        pub title: RefCell<String>,
        #[property(get, set)]
        pub host: RefCell<String>,
        #[property(get, set)]
        pub port: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VncPreferences {
        const NAME: &'static str = "VncPreferences";
        type Type = super::VncPreferences;
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
    impl ObjectImpl for VncPreferences {}
    impl WidgetImpl for VncPreferences {}
    impl PreferencesPageImpl for VncPreferences {}
}

glib::wrapper! {
    pub struct VncPreferences(ObjectSubclass<imp::VncPreferences>)
        @extends gtk::Widget, adw::PreferencesPage;
}

impl VncPreferences {
    pub fn new(existing_configuration: Option<&ConnectionConfiguration>) -> Self {
        let slf: Self = glib::Object::builder().build();

        if let Some(existing_configuration) = existing_configuration.cloned() {
            glib::spawn_future_local(clone!(
                #[weak]
                slf,
                async move {
                    if let Some(v) = existing_configuration.title() {
                        slf.set_title(v);
                    }
                    if let Some(v) = existing_configuration.host() {
                        slf.set_host(v);
                    }
                    if let Some(v) = existing_configuration.port() {
                        slf.set_port(v.to_string());
                    }
                    if let Some(v) = existing_configuration.user() {
                        slf.credentials().set_user(v);
                    }
                    if let Ok(Some(v)) = existing_configuration.password().await {
                        slf.credentials().set_password(v.unsecure());
                    }

                    slf.imp()
                        .credentials
                        .propagate_settings(&existing_configuration)
                        .await;
                }
            ));
        }
        slf
    }
    pub fn credentials(&self) -> &VncCredentialPreferences {
        &self.imp().credentials
    }

    pub fn port_entry_error(&self, error: bool) {
        if error {
            self.imp().port_entry.add_css_class("error");
        } else {
            self.imp().port_entry.remove_css_class("error");
        }
    }
}

#[gtk::template_callbacks]
impl VncPreferences {}
