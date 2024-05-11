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

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;

use crate::connection::types::ConnectionConfiguration;
use crate::connection::vnc::preferences::VncConfiguration;
use crate::save_credentials_button::FieldMonitorSaveCredentialsButton;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
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

    impl ObjectImpl for VncCredentialPreferences {}
    impl WidgetImpl for VncCredentialPreferences {}
    impl PreferencesGroupImpl for VncCredentialPreferences {}
}

glib::wrapper! {
    pub struct VncCredentialPreferences(ObjectSubclass<imp::VncCredentialPreferences>)
        @extends gtk::Widget, adw::PreferencesGroup;
}

impl VncCredentialPreferences {
    pub fn new(existing_configuration: Option<&ConnectionConfiguration>) -> Self {
        let slf: Self = glib::Object::builder().build();

        if let Some(existing_configuration) = existing_configuration {
            if let Some(v) = existing_configuration.user() {
                slf.set_user(v);
            }
            if let Some(v) = existing_configuration.password() {
                slf.set_password(v);
            }
        }
        slf
    }

    pub fn set_user(&self, value: &str) {
        self.imp().user_entry.set_text(value);
    }

    pub fn set_password(&self, value: &str) {
        self.imp().password_entry.set_text(value);
    }
}

#[gtk::template_callbacks]
impl VncCredentialPreferences {}
