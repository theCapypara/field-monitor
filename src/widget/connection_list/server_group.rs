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

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::object::ObjectExt;
use gtk::glib;

use libfieldmonitor::connection::*;

use crate::application::FieldMonitorApplication;
use crate::widget::connection_list::make_server_prefix_suffix;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorServerGroup)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/connection_list/server_group.ui")]
    pub struct FieldMonitorServerGroup {
        #[template_child]
        pub server_title_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub prefix_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub suffix_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub servers: TemplateChild<gtk::ListBox>,
        #[property(get, set)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        #[property(get, set)]
        pub server_title: RefCell<String>,
        #[property(get, set)]
        pub server_subtitle: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorServerGroup {
        const NAME: &'static str = "FieldMonitorServerGroup";
        type Type = super::FieldMonitorServerGroup;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorServerGroup {}
    impl WidgetImpl for FieldMonitorServerGroup {}
    impl BinImpl for FieldMonitorServerGroup {}
}

glib::wrapper! {
    pub struct FieldMonitorServerGroup(ObjectSubclass<imp::FieldMonitorServerGroup>)
        @extends gtk::Widget, adw::Bin;
}

impl FieldMonitorServerGroup {
    pub async fn new(
        app: &FieldMonitorApplication,
        title_server: Option<(&dyn ServerConnection, &[String])>,
    ) -> ConnectionResult<Self> {
        let slf: FieldMonitorServerGroup =
            glib::Object::builder().property("application", app).build();

        if let Some((title_server, full_path)) = title_server {
            let metadata = title_server.metadata();
            slf.set_server_title(metadata.title);
            if let Some(subtitle) = metadata.subtitle {
                slf.set_server_subtitle(subtitle);
            }

            let (prefix, suffix) =
                make_server_prefix_suffix(title_server, full_path, None::<&adw::ActionRow>).await?;
            slf.imp().prefix_box.append(&prefix);
            slf.imp().suffix_box.append(&suffix);
        } else {
            slf.imp().server_title_box.set_visible(false);
        }

        Ok(slf)
    }

    pub fn add(&self, row: &impl IsA<gtk::Widget>) {
        self.imp().servers.append(row);
    }
}
