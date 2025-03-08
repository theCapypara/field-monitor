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
use num_enum::TryFromPrimitive;

use libfieldmonitor::connection::ConnectionConfiguration;
use libfieldmonitor::impl_primitive_enum_param_spec;

use crate::preferences::DebugConfiguration;

#[derive(Copy, Clone, Debug, Default, TryFromPrimitive)]
#[repr(u32)]
pub enum DebugBehaviour {
    #[default]
    Ok = 0,
    AuthError = 1,
    GeneralError = 2,
}

impl_primitive_enum_param_spec!(DebugBehaviour, u32);

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::DebugBehaviourPreferences)]
    #[template(resource = "/de/capypara/FieldMonitor/connection/debug/behaviour_preferences.ui")]
    pub struct DebugBehaviourPreferences {
        #[property(get, set)]
        pub load_metadata_behaviour: RefCell<DebugBehaviour>,

        #[property(get, set)]
        pub load_servers_behaviour: RefCell<DebugBehaviour>,

        #[property(get, set)]
        pub connect_behaviour: RefCell<DebugBehaviour>,

        #[property(get, set)]
        pub store_session: RefCell<String>,

        #[property(get, set)]
        pub store_persistent: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DebugBehaviourPreferences {
        const NAME: &'static str = "DebugBehaviourPreferences";
        type Type = super::DebugBehaviourPreferences;
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
    impl ObjectImpl for DebugBehaviourPreferences {}
    impl WidgetImpl for DebugBehaviourPreferences {}
    impl PreferencesGroupImpl for DebugBehaviourPreferences {}
}

glib::wrapper! {
    pub struct DebugBehaviourPreferences(ObjectSubclass<imp::DebugBehaviourPreferences>)
        @extends gtk::Widget, adw::PreferencesGroup;
}

impl DebugBehaviourPreferences {
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
        self.set_load_servers_behaviour(existing_configuration.load_servers_behaviour());
        self.set_connect_behaviour(existing_configuration.connect_behaviour());
        self.set_store_session(existing_configuration.store_session());
        self.set_store_persistent(existing_configuration.store_persistent());
    }
}

#[gtk::template_callbacks]
impl DebugBehaviourPreferences {}
