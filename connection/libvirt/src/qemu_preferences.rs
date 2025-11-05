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

use std::cell::Cell;
use std::cell::RefCell;

use adw::subclass::prelude::*;
use gtk::gio::{PropertyAction, SimpleActionGroup};
use gtk::glib;
use gtk::prelude::*;
use num_enum::TryFromPrimitive;

use libfieldmonitor::connection::{ConfigAccess, ConfigAccessMut, ConnectionConfiguration};
use libfieldmonitor::impl_primitive_enum_param_spec;

pub(super) trait LibvirtQemuConfiguration {
    fn title(&self) -> Option<&str>;
    fn set_title(&mut self, value: &str);
    fn user_session(&self) -> bool;
    fn set_user_session(&mut self, value: bool);
    fn use_ssh(&self) -> bool;
    fn set_use_ssh(&mut self, value: bool);
    fn ssh_username(&self) -> &str;
    fn set_ssh_username(&mut self, value: &str);
    fn ssh_hostname(&self) -> &str;
    fn set_ssh_hostname(&mut self, value: &str);
}

impl LibvirtQemuConfiguration for ConnectionConfiguration {
    fn title(&self) -> Option<&str> {
        self.get_try_as_str("title")
    }

    fn set_title(&mut self, value: &str) {
        self.set_value("title", value);
    }

    fn user_session(&self) -> bool {
        self.get_try_as_bool("user-session").unwrap_or_default()
    }

    fn set_user_session(&mut self, value: bool) {
        self.set_value("user-session", value)
    }

    fn use_ssh(&self) -> bool {
        self.get_try_as_bool("use-ssh").unwrap_or_default()
    }

    fn set_use_ssh(&mut self, value: bool) {
        self.set_value("use-ssh", value)
    }

    fn ssh_username(&self) -> &str {
        self.get_try_as_str("ssh-username").unwrap_or_default()
    }

    fn set_ssh_username(&mut self, value: &str) {
        self.set_value("ssh-username", value);
    }

    fn ssh_hostname(&self) -> &str {
        self.get_try_as_str("ssh-hostname").unwrap_or_default()
    }

    fn set_ssh_hostname(&mut self, value: &str) {
        self.set_value("ssh-hostname", value);
    }
}

#[derive(Copy, Clone, Debug, Default, TryFromPrimitive, Eq, PartialEq)]
#[repr(u32)]
pub enum SessionType {
    #[default]
    System = 0,
    User = 1,
}

impl_primitive_enum_param_spec!(SessionType, u32);

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::LibvirtQemuPreferences)]
    #[template(resource = "/de/capypara/FieldMonitor/connection/libvirt/qemu_preferences.ui")]
    pub struct LibvirtQemuPreferences {
        #[template_child]
        pub(super) radio_session_system: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) radio_session_user: TemplateChild<gtk::CheckButton>,

        #[property(get, set)]
        pub title: RefCell<String>,
        #[property(get, set)]
        pub session_type: Cell<SessionType>,
        #[property(get, set)]
        pub use_ssh: Cell<bool>,
        #[property(get, set)]
        pub ssh_hostname: RefCell<String>,
        #[property(get, set)]
        pub ssh_username: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibvirtQemuPreferences {
        const NAME: &'static str = "LibvirtQemuPreferences";
        type Type = super::LibvirtQemuPreferences;
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
    impl ObjectImpl for LibvirtQemuPreferences {}
    impl WidgetImpl for LibvirtQemuPreferences {}
    impl PreferencesPageImpl for LibvirtQemuPreferences {}
}

glib::wrapper! {
    pub struct LibvirtQemuPreferences(ObjectSubclass<imp::LibvirtQemuPreferences>)
        @extends gtk::Widget, adw::PreferencesPage,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl LibvirtQemuPreferences {
    pub fn new(config: Option<&ConnectionConfiguration>) -> Self {
        let slf: Self = glib::Object::builder().build();

        let imp = slf.imp();
        let action = PropertyAction::new("session-type", &slf, "session-type");
        let act_grp = SimpleActionGroup::new();
        act_grp.add_action(&action);
        slf.insert_action_group("libvirt-qemu-preferences", Some(&act_grp));

        imp.radio_session_system
            .set_action_name(Some("libvirt-qemu-preferences.session-type"));
        imp.radio_session_system
            .set_action_target(Some(&0_u32.to_variant()));
        imp.radio_session_user
            .set_action_name(Some("libvirt-qemu-preferences.session-type"));
        imp.radio_session_user
            .set_action_target(Some(&1_u32.to_variant()));

        if let Some(config) = config {
            if let Some(title) = config.title() {
                slf.set_title(title);
            }
            slf.set_session_type(if config.user_session() {
                SessionType::User
            } else {
                SessionType::System
            });
            slf.set_use_ssh(config.use_ssh());
            slf.set_ssh_hostname(config.ssh_hostname());
            slf.set_ssh_username(config.ssh_username());
        }

        slf
    }
}

#[gtk::template_callbacks]
impl LibvirtQemuPreferences {}
