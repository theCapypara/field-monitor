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
use std::sync::atomic::AtomicBool;

use adw::subclass::prelude::*;
use glib::clone;
use gtk::gio::{PropertyAction, SimpleActionGroup};
use gtk::glib;
use gtk::prelude::*;
use num_enum::TryFromPrimitive;

use libfieldmonitor::connection::{ConfigAccess, ConfigAccessMut, ConnectionConfiguration};
use libfieldmonitor::impl_primitive_enum_param_spec;

use crate::behaviour_preferences::{DebugBehaviour, DebugBehaviourPreferences};

pub(super) trait DebugConfiguration {
    fn title(&self) -> &str;
    fn set_title(&mut self, value: &str);
    fn mode(&self) -> DebugMode;
    fn set_mode(&mut self, value: DebugMode);
    fn load_servers_behaviour(&self) -> DebugBehaviour;
    fn set_load_servers_behaviour(&mut self, value: DebugBehaviour);
    fn connect_behaviour(&self) -> DebugBehaviour;
    fn set_connect_behaviour(&mut self, value: DebugBehaviour);
    fn store_session(&self) -> &str;
    fn set_store_session(&mut self, value: &str);
    fn store_persistent(&self) -> &str;
    fn set_store_persistent(&mut self, value: &str);
    fn vnc_adapter_enable(&self) -> bool;
    fn set_vnc_adapter_enable(&mut self, value: bool);
    fn vnc_host(&self) -> &str;
    fn set_vnc_host(&mut self, value: &str);
    fn vnc_user(&self) -> &str;
    fn set_vnc_user(&mut self, value: &str);
    fn vnc_password(&self) -> &str;
    fn set_vnc_password(&mut self, value: &str);
    fn rdp_adapter_enable(&self) -> bool;
    fn set_rdp_adapter_enable(&mut self, value: bool);
    fn rdp_host(&self) -> &str;
    fn set_rdp_host(&mut self, value: &str);
    fn rdp_user(&self) -> &str;
    fn set_rdp_user(&mut self, value: &str);
    fn rdp_password(&self) -> &str;
    fn set_rdp_password(&mut self, value: &str);
    fn spice_adapter_enable(&self) -> bool;
    fn set_spice_adapter_enable(&mut self, value: bool);
    fn spice_host(&self) -> &str;
    fn set_spice_host(&mut self, value: &str);
    fn spice_password(&self) -> &str;
    fn set_spice_password(&mut self, value: &str);
    fn vte_adapter_enable(&self) -> bool;
    fn set_vte_adapter_enable(&mut self, value: bool);
    fn custom_adapter_enable(&self) -> bool;
    fn set_custom_adapter_enable(&mut self, value: bool);
    fn custom_overlayed(&self) -> bool;
    fn set_custom_overlayed(&mut self, value: bool);
}

impl DebugConfiguration for ConnectionConfiguration {
    fn title(&self) -> &str {
        self.get_try_as_str("title").unwrap_or_default()
    }

    fn set_title(&mut self, value: &str) {
        self.set_value("title", value);
    }

    fn mode(&self) -> DebugMode {
        self.get_try_as_u32("mode")
            .unwrap_or_default()
            .try_into()
            .unwrap_or_default()
    }

    fn set_mode(&mut self, value: DebugMode) {
        self.set_value("mode", value as u64);
    }

    fn load_servers_behaviour(&self) -> DebugBehaviour {
        self.get_try_as_u32("load-servers-behaviour")
            .unwrap_or_default()
            .try_into()
            .unwrap_or_default()
    }

    fn set_load_servers_behaviour(&mut self, value: DebugBehaviour) {
        self.set_value("load-servers-behaviour", value as u64);
    }

    fn connect_behaviour(&self) -> DebugBehaviour {
        self.get_try_as_u32("connect-behaviour")
            .unwrap_or_default()
            .try_into()
            .unwrap_or_default()
    }

    fn set_connect_behaviour(&mut self, value: DebugBehaviour) {
        self.set_value("connect-behaviour", value as u64);
    }

    fn store_session(&self) -> &str {
        self.get_try_as_str("store-session").unwrap_or_default()
    }

    fn set_store_session(&mut self, value: &str) {
        self.set_value("store-session", value);
    }

    fn store_persistent(&self) -> &str {
        self.get_try_as_str("store-persistent").unwrap_or_default()
    }

    fn set_store_persistent(&mut self, value: &str) {
        self.set_value("store-persistent", value);
    }

    fn vnc_adapter_enable(&self) -> bool {
        self.get_try_as_bool("vnc-adapter-enable")
            .unwrap_or_default()
    }

    fn set_vnc_adapter_enable(&mut self, value: bool) {
        self.set_value("vnc-adapter-enable", value);
    }

    fn vnc_host(&self) -> &str {
        self.get_try_as_str("vnc-host").unwrap_or_default()
    }

    fn set_vnc_host(&mut self, value: &str) {
        self.set_value("vnc-host", value);
    }

    fn vnc_user(&self) -> &str {
        self.get_try_as_str("vnc-user").unwrap_or_default()
    }

    fn set_vnc_user(&mut self, value: &str) {
        self.set_value("vnc-user", value);
    }

    fn vnc_password(&self) -> &str {
        self.get_try_as_str("vnc-password").unwrap_or_default()
    }

    fn set_vnc_password(&mut self, value: &str) {
        self.set_value("vnc-password", value);
    }

    fn rdp_adapter_enable(&self) -> bool {
        self.get_try_as_bool("rdp-adapter-enable")
            .unwrap_or_default()
    }

    fn set_rdp_adapter_enable(&mut self, value: bool) {
        self.set_value("rdp-adapter-enable", value);
    }

    fn rdp_host(&self) -> &str {
        self.get_try_as_str("rdp-host").unwrap_or_default()
    }

    fn set_rdp_host(&mut self, value: &str) {
        self.set_value("rdp-host", value);
    }

    fn rdp_user(&self) -> &str {
        self.get_try_as_str("rdp-user").unwrap_or_default()
    }

    fn set_rdp_user(&mut self, value: &str) {
        self.set_value("rdp-user", value);
    }

    fn rdp_password(&self) -> &str {
        self.get_try_as_str("rdp-password").unwrap_or_default()
    }

    fn set_rdp_password(&mut self, value: &str) {
        self.set_value("rdp-password", value);
    }

    fn spice_adapter_enable(&self) -> bool {
        self.get_try_as_bool("spice-adapter-enable")
            .unwrap_or_default()
    }

    fn set_spice_adapter_enable(&mut self, value: bool) {
        self.set_value("spice-adapter-enable", value);
    }

    fn spice_host(&self) -> &str {
        self.get_try_as_str("spice-host").unwrap_or_default()
    }

    fn set_spice_host(&mut self, value: &str) {
        self.set_value("spice-host", value);
    }

    fn spice_password(&self) -> &str {
        self.get_try_as_str("spice-password").unwrap_or_default()
    }

    fn set_spice_password(&mut self, value: &str) {
        self.set_value("spice-password", value);
    }

    fn vte_adapter_enable(&self) -> bool {
        self.get_try_as_bool("vte-adapter-enable")
            .unwrap_or_default()
    }

    fn set_vte_adapter_enable(&mut self, value: bool) {
        self.set_value("vte-adapter-enable", value);
    }

    fn custom_adapter_enable(&self) -> bool {
        self.get_try_as_bool("custom-adapter-enable")
            .unwrap_or_default()
    }

    fn set_custom_adapter_enable(&mut self, value: bool) {
        self.set_value("custom-adapter-enable", value);
    }

    fn custom_overlayed(&self) -> bool {
        self.get_try_as_bool("custom-overlayed").unwrap_or_default()
    }

    fn set_custom_overlayed(&mut self, value: bool) {
        self.set_value("custom-overlayed", value);
    }
}

#[derive(Copy, Clone, Debug, Default, TryFromPrimitive)]
#[repr(u32)]
pub enum DebugMode {
    #[default]
    Single = 0,
    Multi = 1,
    Complex = 2,
    NoServers = 255, // TODO
}

impl_primitive_enum_param_spec!(DebugMode, u32);

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::DebugPreferences)]
    #[template(resource = "/de/capypara/FieldMonitor/connection/debug/preferences.ui")]
    pub struct DebugPreferences {
        #[template_child]
        pub(super) behaviour: TemplateChild<DebugBehaviourPreferences>,
        #[template_child]
        pub(super) radio_mode_single: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) radio_mode_multi: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) radio_mode_complex: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) radio_mode_no_servers: TemplateChild<gtk::CheckButton>,

        #[property(get, set)]
        pub title: RefCell<String>,
        #[property(get, set)]
        pub mode: RefCell<DebugMode>,
        #[property(get, set)]
        pub vnc_adapter_enable: AtomicBool,
        #[property(get, set)]
        pub vnc_host: RefCell<String>,
        #[property(get, set)]
        pub vnc_user: RefCell<String>,
        #[property(get, set)]
        pub vnc_password: RefCell<String>,
        #[property(get, set)]
        pub rdp_adapter_enable: AtomicBool,
        #[property(get, set)]
        pub rdp_host: RefCell<String>,
        #[property(get, set)]
        pub rdp_user: RefCell<String>,
        #[property(get, set)]
        pub rdp_password: RefCell<String>,
        #[property(get, set)]
        pub spice_adapter_enable: AtomicBool,
        #[property(get, set)]
        pub spice_host: RefCell<String>,
        #[property(get, set)]
        pub spice_password: RefCell<String>,
        #[property(get, set)]
        pub vte_adapter_enable: AtomicBool,
        #[property(get, set)]
        pub custom_adapter_enable: AtomicBool,
        #[property(get, set)]
        pub custom_overlayed: AtomicBool,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DebugPreferences {
        const NAME: &'static str = "DebugPreferences";
        type Type = super::DebugPreferences;
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
    impl ObjectImpl for DebugPreferences {}
    impl WidgetImpl for DebugPreferences {}
    impl PreferencesPageImpl for DebugPreferences {}
}

glib::wrapper! {
    pub struct DebugPreferences(ObjectSubclass<imp::DebugPreferences>)
        @extends gtk::Widget, adw::PreferencesPage;
}

impl DebugPreferences {
    pub fn new(existing_configuration: Option<&ConnectionConfiguration>) -> Self {
        let slf: Self = glib::Object::builder().build();

        let imp = slf.imp();
        let action = PropertyAction::new("mode-change", &slf, "mode");
        let act_grp = SimpleActionGroup::new();
        act_grp.add_action(&action);
        slf.insert_action_group("debug-preferences", Some(&act_grp));
        imp.radio_mode_single
            .set_action_name(Some("debug-preferences.mode-change"));
        imp.radio_mode_single
            .set_action_target(Some(&0u32.to_variant()));
        imp.radio_mode_multi
            .set_action_name(Some("debug-preferences.mode-change"));
        imp.radio_mode_multi
            .set_action_target(Some(&1u32.to_variant()));
        imp.radio_mode_complex
            .set_action_name(Some("debug-preferences.mode-change"));
        imp.radio_mode_complex
            .set_action_target(Some(&2u32.to_variant()));
        imp.radio_mode_no_servers
            .set_action_name(Some("debug-preferences.mode-change"));
        imp.radio_mode_no_servers
            .set_action_target(Some(&255u32.to_variant()));

        if let Some(existing_configuration) = existing_configuration.cloned() {
            glib::spawn_future_local(clone!(
                #[weak]
                slf,
                async move {
                    slf.set_title(existing_configuration.title());
                    slf.set_mode(existing_configuration.mode());
                    slf.set_vnc_adapter_enable(existing_configuration.vnc_adapter_enable());
                    slf.set_vnc_host(existing_configuration.vnc_host());
                    slf.set_vnc_user(existing_configuration.vnc_user());
                    slf.set_vnc_password(existing_configuration.vnc_password());
                    slf.set_rdp_adapter_enable(existing_configuration.rdp_adapter_enable());
                    slf.set_rdp_host(existing_configuration.rdp_host());
                    slf.set_rdp_user(existing_configuration.rdp_user());
                    slf.set_rdp_password(existing_configuration.rdp_password());
                    slf.set_spice_adapter_enable(existing_configuration.spice_adapter_enable());
                    slf.set_spice_host(existing_configuration.spice_host());
                    slf.set_spice_password(existing_configuration.spice_password());
                    slf.set_vte_adapter_enable(existing_configuration.vte_adapter_enable());
                    slf.set_custom_adapter_enable(existing_configuration.custom_adapter_enable());
                    slf.set_custom_overlayed(existing_configuration.custom_overlayed());

                    slf.imp()
                        .behaviour
                        .propagate_settings(&existing_configuration)
                        .await;
                }
            ));
        }

        slf
    }
    pub fn behaviour(&self) -> &DebugBehaviourPreferences {
        &self.imp().behaviour
    }
}

#[gtk::template_callbacks]
impl DebugPreferences {}
