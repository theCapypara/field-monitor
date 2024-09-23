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

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::object::ObjectExt;
use gtk::gio;
use gtk::glib;

use crate::application::FieldMonitorApplication;
use crate::widget::window::FieldMonitorWindow;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorConnectionView)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/connection_view.ui")]
    pub struct FieldMonitorConnectionView {
        #[template_child]
        pub button_fullscreen: TemplateChild<gtk::Button>,
        #[template_child]
        pub osd_title_revealer: TemplateChild<gtk::Revealer>,
        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        #[property(get, set)]
        pub title: RefCell<String>,
        #[property(get, set)]
        pub subtitle: RefCell<String>,
        #[property(get, set, default = true)]
        pub reveal_osd_controls: AtomicBool,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorConnectionView {
        const NAME: &'static str = "FieldMonitorConnectionView";
        type Type = super::FieldMonitorConnectionView;
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
    impl ObjectImpl for FieldMonitorConnectionView {}
    impl WidgetImpl for FieldMonitorConnectionView {}
    impl BinImpl for FieldMonitorConnectionView {}
}

glib::wrapper! {
    pub struct FieldMonitorConnectionView(ObjectSubclass<imp::FieldMonitorConnectionView>)
        @extends gtk::Widget, adw::Bin,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl FieldMonitorConnectionView {
    pub fn new(app: &FieldMonitorApplication, window: Option<&FieldMonitorWindow>) -> Self {
        let slf: Self = glib::Object::builder().property("application", app).build();
        let imp = slf.imp();

        if let Some(window) = window {
            window.connect_notify_local(
                Some("fullscreened"),
                glib::clone!(
                    #[weak]
                    slf,
                    move |window, _| {
                        if window.is_fullscreen() {
                            slf.imp()
                                .button_fullscreen
                                .set_icon_name("arrows-pointing-inward-symbolic");
                        } else {
                            slf.imp()
                                .button_fullscreen
                                .set_icon_name("arrows-pointing-outward-symbolic");
                        }
                    }
                ),
            );

            // Setter doesn't work so well here.
            window.mobile_breakpoint().add_setter(
                &*imp.osd_title_revealer,
                "reveal-child",
                Some(&false.into()),
            );
        }

        slf
    }
}

#[gtk::template_callbacks]
impl FieldMonitorConnectionView {}
