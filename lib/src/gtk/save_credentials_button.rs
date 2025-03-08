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

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorSaveCredentialsButton)]
    #[template(resource = "/de/capypara/FieldMonitor/lib/gtk/save_credentials_button.ui")]
    pub struct FieldMonitorSaveCredentialsButton {
        #[template_child]
        pub popover: TemplateChild<gtk::Popover>,
        #[template_child]
        pub button: TemplateChild<gtk::Button>,
        #[template_child]
        pub save_and_remember: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub ask_every_time: TemplateChild<gtk::CheckButton>,

        #[property(get, set, construct, default = true)]
        pub save_password: Cell<bool>,

        pub _ignore_toggles: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorSaveCredentialsButton {
        const NAME: &'static str = "FieldMonitorSaveCredentialsButton";
        type Type = super::FieldMonitorSaveCredentialsButton;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorSaveCredentialsButton {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().on_notify_self_save_password();
        }
    }
    impl WidgetImpl for FieldMonitorSaveCredentialsButton {}
    impl BoxImpl for FieldMonitorSaveCredentialsButton {}
}

glib::wrapper! {
    pub struct FieldMonitorSaveCredentialsButton(ObjectSubclass<imp::FieldMonitorSaveCredentialsButton>)
        @extends gtk::Widget, gtk::Box;
}

impl Default for FieldMonitorSaveCredentialsButton {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldMonitorSaveCredentialsButton {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

#[gtk::template_callbacks]
impl FieldMonitorSaveCredentialsButton {
    #[template_callback]
    fn on_notify_self_save_password(&self) {
        self.imp()._ignore_toggles.set(true);
        if self.save_password() {
            self.imp().button.set_icon_name("key-symbolic");
            self.imp().ask_every_time.set_active(false);
            self.imp().save_and_remember.set_active(true);
        } else {
            self.imp().button.set_icon_name("key-off-symbolic");
            self.imp().ask_every_time.set_active(true);
            self.imp().save_and_remember.set_active(false);
        }
        self.imp()._ignore_toggles.set(false);
    }

    #[template_callback]
    fn on_button_clicked(&self) {
        self.imp().popover.popup()
    }

    #[template_callback]
    fn on_save_and_remember_toggled(&self) {
        if self.imp()._ignore_toggles.get() {
            return;
        }
        self.set_save_password(true);
        self.imp().popover.popdown();
    }

    #[template_callback]
    fn on_ask_every_time_toggled(&self) {
        if self.imp()._ignore_toggles.get() {
            return;
        }
        self.set_save_password(false);
        self.imp().popover.popdown();
    }
}
