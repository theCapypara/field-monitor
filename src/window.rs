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

use crate::connections::FieldMonitorConnections;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/de/capypara/FieldMonitor/window.ui")]
    pub struct FieldMonitorWindow {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub tab_bar: TemplateChild<adw::TabBar>,
        #[template_child]
        pub tab_view: TemplateChild<adw::TabView>,
        #[template_child]
        pub overview: TemplateChild<adw::TabOverview>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorWindow {
        const NAME: &'static str = "FieldMonitorWindow";
        type Type = super::FieldMonitorWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FieldMonitorWindow {}
    impl WidgetImpl for FieldMonitorWindow {}
    impl WindowImpl for FieldMonitorWindow {}
    impl ApplicationWindowImpl for FieldMonitorWindow {}
    impl AdwApplicationWindowImpl for FieldMonitorWindow {}
}

glib::wrapper! {
    pub struct FieldMonitorWindow(ObjectSubclass<imp::FieldMonitorWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl FieldMonitorWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        let slf: Self = glib::Object::builder()
            .property("application", application)
            .build();
        slf
    }
}

#[gtk::template_callbacks]
impl FieldMonitorWindow {
    #[template_callback]
    fn on_tab_view_create_window(&self, _tab_view: &adw::TabView) -> adw::TabView {
        let new_window = FieldMonitorWindow::new(&self.application().unwrap());
        new_window.present();
        new_window.imp().tab_view.clone()
    }

    #[template_callback]
    fn on_button_overview_clicked(&self, _button: &gtk::Button) {
        self.imp().overview.set_open(true);
    }

    #[template_callback]
    fn on_button_home_clicked(&self, _button: &gtk::Button) {
        let page = self
            .get_open_connection_list_page()
            .unwrap_or_else(|| self.open_new_connection_list());
        self.imp().tab_view.set_selected_page(&page);
    }

    #[template_callback]
    fn on_tab_view_page_attached(&self, _page: &adw::TabPage, _position: i32) {
        self.configure_tab_bar_autohide();
    }

    #[template_callback]
    fn on_tab_view_page_detached(&self, _page: &adw::TabPage, _position: i32) {
        self.configure_tab_bar_autohide();
        if self.imp().tab_view.n_pages() == 0 {
            self.open_new_connection_list();
        }
    }
}

impl FieldMonitorWindow {
    fn get_open_connection_list_page(&self) -> Option<adw::TabPage> {
        for child in self.imp().tab_view.pages().iter::<adw::TabPage>() {
            let child = child.unwrap(); // TODO: maybe want to be more graceful? but this really should never happen.
            if child
                .child()
                .type_()
                .is_a(FieldMonitorConnections::static_type())
            {
                return Some(child);
            }
        }
        None
    }

    fn configure_tab_bar_autohide(&self) {
        if self.imp().tab_view.n_pages() == 1 {
            // If we only have the connection list open: hide tab bar
            self.imp()
                .tab_bar
                .set_autohide(self.get_open_connection_list_page().is_some());
        }
    }

    pub fn open_new_connection_list(&self) -> adw::TabPage {
        let title = gettext("Connection List");
        let page = FieldMonitorConnections::new();
        let tab_page = self.imp().tab_view.append(&page);

        tab_page.set_title(&title);
        tab_page.set_live_thumbnail(true);
        tab_page
    }
}
