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

use std::cell::RefCell;

use adw::subclass::prelude::*;
use glib::clone;
use gtk::glib;
use gtk::prelude::*;

use crate::quick_connect::QuickConnectConfig;
use libfieldmonitor::connection::ConnectionConfiguration;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::QuickConnectPreferences)]
    #[template(resource = "/de/capypara/FieldMonitor/quick_connect/preferences.ui")]
    pub struct QuickConnectPreferences {
        #[property(get, set)]
        pub user: RefCell<String>,

        #[property(get, set)]
        pub password: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for QuickConnectPreferences {
        const NAME: &'static str = "QuickConnectPreferences";
        type Type = super::QuickConnectPreferences;
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
    impl ObjectImpl for QuickConnectPreferences {}
    impl WidgetImpl for QuickConnectPreferences {}
    impl PreferencesGroupImpl for QuickConnectPreferences {}
}

glib::wrapper! {
    pub struct QuickConnectPreferences(ObjectSubclass<imp::QuickConnectPreferences>)
        @extends gtk::Widget, adw::PreferencesGroup,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl QuickConnectPreferences {
    pub fn new(existing_configuration: Option<&ConnectionConfiguration>) -> Self {
        let slf: Self = glib::Object::builder().build();

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
        if let Some(value) = existing_configuration.user() {
            self.set_user(value);
        }
        if let Some(value) = existing_configuration.password() {
            self.set_password(value.unsecure());
        }
    }
}

#[gtk::template_callbacks]
impl QuickConnectPreferences {}
