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
use adw::prelude::*;
use gtk::subclass::prelude::*;
use gtk::TemplateChild;
use libfieldmonitor::impl_primitive_enum_param_spec;
use num_enum::TryFromPrimitive;
use std::cell::RefCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, TryFromPrimitive)]
#[repr(u32)]
pub enum IsOnline {
    #[default]
    Unknown = 0,
    Online = 1,
    Offline = 2,
}
impl_primitive_enum_param_spec!(IsOnline, u32);

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::ServerInfoIcon)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/connection_list/server_info/icon.ui")]
    pub struct ServerInfoIcon {
        #[template_child]
        pub child_wdg: TemplateChild<adw::Bin>,
        #[template_child]
        pub icon: TemplateChild<adw::Bin>,
        #[property(get, set)]
        pub status: RefCell<IsOnline>,
        #[property(get, set)]
        pub child: RefCell<Option<gtk::Widget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServerInfoIcon {
        const NAME: &'static str = "ServerInfoIcon";
        type Type = super::ServerInfoIcon;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServerInfoIcon {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.connect_status_notify(glib::clone!(move |obj| obj.on_set_status()));
        }
    }
    impl WidgetImpl for ServerInfoIcon {}
    impl BoxImpl for ServerInfoIcon {}
}

glib::wrapper! {
    pub struct ServerInfoIcon(ObjectSubclass<imp::ServerInfoIcon>)
        @extends gtk::Widget, gtk::Box;
}

impl Default for ServerInfoIcon {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerInfoIcon {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    fn on_set_status(&self) {
        let imp = self.imp();
        match self.status() {
            IsOnline::Unknown => {
                if imp.icon.child().and_downcast::<gtk::Box>().is_none() {
                    let placeholder = gtk::Box::builder().width_request(8).build();
                    imp.icon.set_child(Some(&placeholder));
                }
            }
            status => {
                let (class, tooltip_text, icon_name) = if status == IsOnline::Online {
                    ("success", "Online", "circle-filled-symbolic")
                } else {
                    ("dim-label", "Offline", "circle-outline-thick-symbolic")
                };

                if let Some(icon) = imp.icon.child().and_downcast::<gtk::Image>() {
                    icon.set_tooltip_text(Some(tooltip_text));
                    icon.set_icon_name(Some(icon_name));
                    icon.remove_css_class("dim-label");
                    icon.remove_css_class("success");
                    icon.add_css_class(class);
                } else {
                    let status_icon = gtk::Image::builder()
                        .pixel_size(8)
                        .icon_name(icon_name)
                        .css_classes([class])
                        .tooltip_text(tooltip_text)
                        .valign(gtk::Align::Center)
                        .build();
                    imp.icon.set_child(Some(&status_icon));
                }
            }
        }
    }
}
