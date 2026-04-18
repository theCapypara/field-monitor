/* Copyright 2024-2026 Marco Köpcke
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

use crate::application::FieldMonitorApplication;
use crate::widget::connection_view::FieldMonitorServerScreen;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use libfieldmonitor::adapter::types::AdapterDisplay;
use log::debug;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/single_screen_window.ui")]
    pub struct FieldMonitorSingleScreenWindow {
        #[template_child]
        pub content_bin: TemplateChild<adw::Bin>,
        pub screen: RefCell<Option<FieldMonitorServerScreen>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorSingleScreenWindow {
        const NAME: &'static str = "FieldMonitorSingleScreenWindow";
        type Type = super::FieldMonitorSingleScreenWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FieldMonitorSingleScreenWindow {}
    impl WidgetImpl for FieldMonitorSingleScreenWindow {}
    impl WindowImpl for FieldMonitorSingleScreenWindow {}
    impl ApplicationWindowImpl for FieldMonitorSingleScreenWindow {}
    impl AdwApplicationWindowImpl for FieldMonitorSingleScreenWindow {}
}

glib::wrapper! {
    pub struct FieldMonitorSingleScreenWindow(ObjectSubclass<imp::FieldMonitorSingleScreenWindow>)
        @extends gtk::Widget, adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window,
        @implements gio::ActionMap, gio::ActionGroup, gtk::ShortcutManager, gtk::Root, gtk::Native, gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl FieldMonitorSingleScreenWindow {
    pub fn new(
        app: &FieldMonitorApplication,
        primary_screen: &FieldMonitorServerScreen,
        monitor_index: u32,
        display: Box<dyn AdapterDisplay>,
        title: &str,
        subtitle: &str,
    ) -> Self {
        let slf: Self = glib::Object::builder().property("application", app).build();

        // Set up the fullscreen action as a PropertyAction on "fullscreened".
        slf.add_action(&gio::PropertyAction::new(
            "fullscreen",
            &slf,
            "fullscreened",
        ));

        // Create a secondary monitor screen using the existing FieldMonitorServerScreen.
        let screen = FieldMonitorServerScreen::new_secondary_monitor(
            app,
            &primary_screen.server_path(),
            &primary_screen.adapter_id(),
            monitor_index,
            display,
            title,
            subtitle,
        );

        // Set up close callback: closing the screen closes this window.
        screen.set_close_cb(glib::clone!(
            #[weak(rename_to = win)]
            slf,
            move || {
                win.close();
            }
        ));

        // Bind fullscreen icon and header bar overlay behavior to this window.
        screen.setup_window_bindings(&slf);

        // Set window title to match the screen's monitor-specific title.
        slf.set_title(Some(&screen.title()));

        slf.imp().content_bin.set_child(Some(&screen));
        slf.imp().screen.replace(Some(screen));

        // Auto-close when the primary connection disconnects.
        primary_screen.connect_connected_notify(glib::clone!(
            #[weak(rename_to = win)]
            slf,
            move |primary| {
                if !primary.connected() {
                    debug!("Primary screen disconnected, closing single screen window");
                    win.close();
                }
            }
        ));

        // Auto-close when the primary screen is destroyed.
        primary_screen.connect_unrealize(glib::clone!(
            #[weak(rename_to = win)]
            slf,
            move |_| {
                debug!("Primary screen unrealized, closing single screen window");
                win.close();
            }
        ));

        slf
    }
}
