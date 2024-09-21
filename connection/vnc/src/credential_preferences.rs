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

use std::cell::{Cell, RefCell};

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;

use libfieldmonitor::connection::ConnectionConfiguration;
use libfieldmonitor::gtk::FieldMonitorSaveCredentialsButton;

use crate::preferences::VncConfiguration;
use crate::util::clear_editable_if_becoming_not_editable;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::VncCredentialPreferences)]
    #[template(resource = "/de/capypara/FieldMonitor/connection/vnc/credential_preferences.ui")]
    pub struct VncCredentialPreferences {
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
    impl ObjectSubclass for VncCredentialPreferences {
        const NAME: &'static str = "VncCredentialPreferences";
        type Type = super::VncCredentialPreferences;
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
    impl ObjectImpl for VncCredentialPreferences {
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
    impl WidgetImpl for VncCredentialPreferences {}
    impl PreferencesGroupImpl for VncCredentialPreferences {}
}

glib::wrapper! {
    pub struct VncCredentialPreferences(ObjectSubclass<imp::VncCredentialPreferences>)
        @extends gtk::Widget, adw::PreferencesGroup;
}

impl VncCredentialPreferences {
    pub fn new(
        existing_configuration: Option<&ConnectionConfiguration>,
        use_temporary_credentials: bool,
    ) -> Self {
        let slf: Self = glib::Object::builder()
            .property("use-temporary-credentials", use_temporary_credentials)
            .build();

        if let Some(existing_configuration) = existing_configuration.cloned() {
            glib::spawn_future_local(clone!(
                #[weak]
                slf,
                async move {
                    slf.propagate_settings(&existing_configuration).await;
                }
            ));
        }
        slf
    }

    pub async fn propagate_settings(&self, existing_configuration: &ConnectionConfiguration) {
        if let Some(v) = existing_configuration.user() {
            self.set_user(v);
        }
        if let Ok(Some(v)) = existing_configuration.password().await {
            self.set_password(v.unsecure());
        }
    }

    pub fn user_if_remembered(&self) -> Option<String> {
        if self.imp().user_entry_save_button.save_password() {
            Some(self.user())
        } else {
            None
        }
    }

    pub fn password_if_remembered(&self) -> Option<String> {
        if self.imp().password_entry_save_button.save_password() {
            Some(self.password())
        } else {
            None
        }
    }
}

#[gtk::template_callbacks]
impl VncCredentialPreferences {}
