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
use std::num::NonZeroU32;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;

use crate::connection::types::ConnectionConfiguration;
use crate::connection::vnc::credential_preferences::VncCredentialPreferences;

pub(super) trait VncConfiguration {
    fn title(&self) -> Option<&str>;
    fn host(&self) -> Option<&str>;
    fn port(&self) -> Option<NonZeroU32>;
    fn user(&self) -> Option<&str>;
    fn password(&self) -> Option<&str>;
}

impl VncConfiguration for ConnectionConfiguration {
    fn title(&self) -> Option<&str> {
        todo!()
    }

    fn host(&self) -> Option<&str> {
        todo!()
    }

    fn port(&self) -> Option<NonZeroU32> {
        todo!()
    }

    fn user(&self) -> Option<&str> {
        todo!()
    }

    fn password(&self) -> Option<&str> {
        todo!()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
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

        if let Some(existing_configuration) = existing_configuration {
            if let Some(v) = existing_configuration.title() {
                slf.imp().title_entry.set_text(v);
            }
            if let Some(v) = existing_configuration.host() {
                slf.imp().host_entry.set_text(v);
            }
            if let Some(v) = existing_configuration.port() {
                slf.imp().port_entry.set_text(&v.to_string());
            }
            if let Some(v) = existing_configuration.user() {
                slf.imp().credentials.set_user(v);
            }
            if let Some(v) = existing_configuration.password() {
                slf.imp().credentials.set_password(v);
            }
        }
        slf
    }
}

#[gtk::template_callbacks]
impl VncPreferences {}
