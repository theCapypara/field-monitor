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

use std::cell::{Cell, RefCell};

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;

use libfieldmonitor::connection::ConnectionConfiguration;
use libfieldmonitor::gtk::FieldMonitorSaveCredentialsButton;

use crate::preferences::GenericGroupConfiguration;
use crate::server_config::FinalizedServerConfig;
use crate::util::clear_editable_if_becoming_not_editable;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::GenericGroupCredentialPreferences)]
    #[template(
        resource = "/de/capypara/FieldMonitor/connection/generic-group/credential_preferences.ui"
    )]
    pub struct GenericGroupCredentialPreferences {
        #[template_child]
        pub(super) user_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) password_entry: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub(super) user_entry_save_button: TemplateChild<FieldMonitorSaveCredentialsButton>,
        #[template_child]
        pub(super) password_entry_save_button: TemplateChild<FieldMonitorSaveCredentialsButton>,

        #[property(get, set)]
        pub user: RefCell<String>,
        #[property(get, set)]
        pub password: RefCell<String>,

        #[property(get, construct_only, default = true)]
        /// If true: If the credentials are set to "ask", then still allow the user
        /// to input a value, if false, do not allow the user to input a value.
        pub use_temporary_credentials: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GenericGroupCredentialPreferences {
        const NAME: &'static str = "GenericGroupCredentialPreferences";
        type Type = super::GenericGroupCredentialPreferences;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for GenericGroupCredentialPreferences {
        fn constructed(&self) {
            self.parent_constructed();
            if !self.use_temporary_credentials.get() {
                self.user_entry_save_button
                    .bind_property("save_password", &*self.user_entry, "editable")
                    .sync_create()
                    .build();
                self.password_entry_save_button
                    .bind_property("save_password", &*self.password_entry, "editable")
                    .sync_create()
                    .build();
                clear_editable_if_becoming_not_editable(&*self.user_entry);
                clear_editable_if_becoming_not_editable(&*self.password_entry);
            }
        }
    }
    impl WidgetImpl for GenericGroupCredentialPreferences {}
    impl PreferencesGroupImpl for GenericGroupCredentialPreferences {}
}

glib::wrapper! {
    pub struct GenericGroupCredentialPreferences(ObjectSubclass<imp::GenericGroupCredentialPreferences>)
        @extends gtk::Widget, adw::PreferencesGroup;
}

impl GenericGroupCredentialPreferences {
    pub fn new(
        server: &str,
        existing_configuration: Option<&ConnectionConfiguration>,
        use_temporary_credentials: bool,
    ) -> Self {
        let slf: Self = glib::Object::builder()
            .property("use-temporary-credentials", use_temporary_credentials)
            .build();

        let server = server.to_string();
        if let Some(existing_configuration) = existing_configuration.cloned() {
            glib::spawn_future_local(clone!(
                #[weak]
                slf,
                async move {
                    slf.propagate_settings(&server, &existing_configuration)
                        .await;
                }
            ));
        }
        slf
    }

    pub async fn propagate_settings<T: GenericGroupConfiguration>(
        &self,
        server: &str,
        existing_configuration: &T,
    ) {
        if let Some(v) = existing_configuration.user(server) {
            self.set_user(v);
        }
        if let Ok(Some(v)) = existing_configuration.password(server).await {
            self.set_password(v.unsecure());
        }
    }

    pub fn update_server_config(&self, config: &mut FinalizedServerConfig) {
        let user = self.user();
        let pass = self.password();
        config.user = if user.is_empty() { None } else { Some(user) };
        config.password = if pass.is_empty() {
            None
        } else {
            Some(pass.into())
        };
        config.user_remember = self.imp().user_entry_save_button.save_password();
        config.password_remember = self.imp().password_entry_save_button.save_password();
    }

    pub fn as_incomplete_server_config(&self) -> FinalizedServerConfig {
        let mut config = FinalizedServerConfig::default();
        self.update_server_config(&mut config);
        config
    }
}

#[gtk::template_callbacks]
impl GenericGroupCredentialPreferences {}
