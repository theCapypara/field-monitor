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
use adw::ToastOverlay;
use gettextrs::gettext;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/de/capypara/FieldMonitor/connection_list.ui")]
    pub struct FieldMonitorConnectionList {
        #[template_child]
        pub toast_overlay: TemplateChild<ToastOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorConnectionList {
        const NAME: &'static str = "FieldMonitorConnectionList";
        type Type = super::FieldMonitorConnectionList;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FieldMonitorConnectionList {}
    impl WidgetImpl for FieldMonitorConnectionList {}
    impl BinImpl for FieldMonitorConnectionList {}
}

glib::wrapper! {
    pub struct FieldMonitorConnectionList(ObjectSubclass<imp::FieldMonitorConnectionList>)
        @extends gtk::Widget, adw::Bin;
}

impl Default for FieldMonitorConnectionList {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldMonitorConnectionList {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
    pub fn connection_added(&self) {
        self.imp().toast_overlay.add_toast(
            adw::Toast::builder()
                .title(gettext("Connection successfully added."))
                .timeout(5)
                .build(),
        )
    }
}

#[gtk::template_callbacks]
impl FieldMonitorConnectionList {}
