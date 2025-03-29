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

use crate::application::AppState;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorAppStatus)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/app_status.ui")]
    pub struct FieldMonitorAppStatus {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[property(get, set)]
        state: RefCell<AppState>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorAppStatus {
        const NAME: &'static str = "FieldMonitorAppStatus";
        type Type = super::FieldMonitorAppStatus;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorAppStatus {}
    impl WidgetImpl for FieldMonitorAppStatus {}
    impl BinImpl for FieldMonitorAppStatus {}
}

glib::wrapper! {
    pub struct FieldMonitorAppStatus(ObjectSubclass<imp::FieldMonitorAppStatus>)
        @extends gtk::Widget, adw::Bin;
}

#[gtk::template_callbacks]
impl FieldMonitorAppStatus {
    #[template_callback]
    fn on_self_state_changed(&self) {
        self.imp().stack.set_visible_child_name(match self.state() {
            AppState::Initializing | AppState::Ready => "initializing",
            AppState::ErrSecretsGeneral => "error-secrets-general",
            AppState::ErrSecretsInvalid => "error-secrets-invalid",
        });
    }
}
