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

use adw::prelude::{ComboRowExt, PreferencesRowExt};
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use gtk::glib;
use gtk::prelude::*;
use secure_string::SecureString;

use libfieldmonitor::connection::ConnectionConfiguration;
use libfieldmonitor::gtk::FieldMonitorSaveCredentialsButton;

use crate::preferences::ProxmoxConfiguration;

const AUTH_MODE_PASSWORD: u32 = 0;
const AUTH_MODE_APIKEY: u32 = 1;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::ProxmoxCredentialPreferences)]
    #[template(resource = "/de/capypara/FieldMonitor/connection/proxmox/credential_preferences.ui")]
    pub struct ProxmoxCredentialPreferences {
        #[template_child]
        pub password_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub password_entry_save_button: TemplateChild<FieldMonitorSaveCredentialsButton>,
        #[template_child]
        pub auth_mode_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub username_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub tokenid_entry: TemplateChild<adw::EntryRow>,
        #[property(get, set, default = "root@pam")]
        username: RefCell<String>,
        #[property(get, set)]
        tokenid: RefCell<String>,
        #[property(get, set)]
        password_or_apikey: RefCell<String>,
        #[property(get, set)]
        use_apikey: Cell<bool>,
        #[property(get, construct_only, default = true)]
        /// If true: If the credentials are set to "ask", then still allow the user
        /// to input a value, if false, do not allow the user to input a value.
        pub use_temporary_credentials: Cell<bool>,

        pub currently_updating_widgets: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProxmoxCredentialPreferences {
        const NAME: &'static str = "ProxmoxCredentialPreferences";
        type Type = super::ProxmoxCredentialPreferences;
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
    impl ObjectImpl for ProxmoxCredentialPreferences {
        fn constructed(&self) {
            self.parent_constructed();
            if !self.use_temporary_credentials.get() {
                self.password_entry_save_button
                    .bind_property("save_password", &*self.password_entry, "editable")
                    .sync_create()
                    .build();
                // Clears the widget if it becomes non-editable
                self.password_entry
                    .connect_notify(Some("editable"), move |w, _| {
                        if !w.is_editable() {
                            w.set_text("")
                        }
                    });
            }
        }
    }
    impl WidgetImpl for ProxmoxCredentialPreferences {}
    impl PreferencesGroupImpl for ProxmoxCredentialPreferences {}
}

glib::wrapper! {
    pub struct ProxmoxCredentialPreferences(ObjectSubclass<imp::ProxmoxCredentialPreferences>)
        @extends gtk::Widget, adw::PreferencesGroup;
}

impl ProxmoxCredentialPreferences {
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
        self.set_username(existing_configuration.username().unwrap_or_default());
        self.set_tokenid(existing_configuration.tokenid().unwrap_or_default());
        self.set_use_apikey(existing_configuration.use_apikey());
        if let Ok(Some(v)) = existing_configuration.password_or_apikey().await {
            self.set_password_or_apikey(v.unsecure());
        }
    }

    pub fn apply_persistent_config(
        &self,
        config: &mut ConnectionConfiguration,
    ) -> Result<(), anyhow::Error> {
        config.set_username(&self.username());
        config.set_tokenid(&self.tokenid());
        config.set_use_apikey(self.use_apikey());
        config.set_password_or_apikey(Some(SecureString::from(self.password_or_apikey())));
        Ok(())
    }

    pub fn apply_session_config(
        &self,
        config: &mut ConnectionConfiguration,
    ) -> Result<(), anyhow::Error> {
        config.set_username(&self.username());
        config.set_tokenid(&self.tokenid());
        config.set_use_apikey(self.use_apikey());
        config.set_password_or_apikey_session(Some(SecureString::from(self.password_or_apikey())));
        Ok(())
    }
}

#[gtk::template_callbacks]
impl ProxmoxCredentialPreferences {
    #[template_callback]
    fn on_self_use_apikey_changed(&self) {
        let new_v = if self.use_apikey() {
            AUTH_MODE_APIKEY
        } else {
            AUTH_MODE_PASSWORD
        };
        if new_v != self.imp().auth_mode_combo.selected() {
            self.imp().auth_mode_combo.set_selected(new_v)
        }
    }

    #[template_callback]
    fn on_auth_mode_combo_selected(&self) {
        let use_apikey = match self.imp().auth_mode_combo.selected() {
            AUTH_MODE_PASSWORD => false,
            AUTH_MODE_APIKEY => true,
            _ => unreachable!(),
        };
        self.set_use_apikey(use_apikey);
        self.imp().tokenid_entry.set_visible(use_apikey);
        self.imp().username_entry.set_visible(!use_apikey);
        self.imp().password_entry.set_title(&if use_apikey {
            gettext("API Key")
        } else {
            gettext("Password")
        });
    }
}
