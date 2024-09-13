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

use std::sync::atomic::AtomicBool;

use adw::prelude::BinExt;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use gtk::glib::Variant;
use gtk::prelude::*;

use crate::application::FieldMonitorApplication;
use crate::widget::connection_list::FieldMonitorConnectionList;

#[cfg(feature = "devel")]
const DEBUG_TABS: bool = true;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorWindow)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/window.ui")]
    pub struct FieldMonitorWindow {
        #[template_child]
        pub main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub connection_list_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub button_fullscreen: TemplateChild<gtk::Button>,
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub tab_view: TemplateChild<adw::TabView>,
        #[template_child]
        pub overview: TemplateChild<adw::TabOverview>,
        #[template_child]
        pub button_search_in_list: TemplateChild<gtk::Button>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[property(get, set)]
        pub connection_list_visible: AtomicBool,
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

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorWindow {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_actions();
        }
    }
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
    pub fn new(application: &FieldMonitorApplication) -> Self {
        let slf: Self = glib::Object::builder()
            .property("application", application)
            .build();

        #[cfg(feature = "devel")]
        slf.add_css_class("devel");
        #[cfg(feature = "devel")]
        if DEBUG_TABS {
            slf.add_debug_tabs();
        }

        let conn_list = FieldMonitorConnectionList::new(application);
        slf.imp().connection_list_bin.set_child(Some(&conn_list));

        conn_list.connect_notify_local(
            Some("search-is-visible"),
            glib::clone!(
                #[weak]
                slf,
                move |conn_list, _| {
                    let value = conn_list.search_is_visible();
                    slf.change_action_state("connection-list-toggle-search", &value.to_variant());
                }
            ),
        );

        slf.on_main_stack_visible_child_name_changed();

        slf
    }

    fn setup_actions(&self) {
        let show_connection_action = gio::ActionEntry::builder("show-connection-list")
            .activate(Self::act_show_connection_list)
            .build();

        let connection_list_toggle_search_action =
            gio::ActionEntry::builder("connection-list-toggle-search")
                .state(false.to_variant())
                .activate(Self::act_connection_list_toggle_search)
                .build();

        self.add_action(&gio::PropertyAction::new(
            "fullscreen",
            self,
            "fullscreened",
        ));

        self.add_action_entries([show_connection_action, connection_list_toggle_search_action]);
    }

    pub fn toast_connection_added(&self) {
        self.imp().toast_overlay.add_toast(
            adw::Toast::builder()
                .title(gettext("Connection successfully added."))
                .timeout(5)
                .build(),
        )
    }

    pub fn toast_connection_updated(&self) {
        self.imp().toast_overlay.add_toast(
            adw::Toast::builder()
                .title(gettext("Connection successfully updated."))
                .timeout(5)
                .build(),
        )
    }

    pub fn toast_connection_removed(&self) {
        self.imp().toast_overlay.add_toast(
            adw::Toast::builder()
                .title(gettext("Connection successfully removed."))
                .timeout(5)
                .build(),
        )
    }
}

#[gtk::template_callbacks]
impl FieldMonitorWindow {
    #[template_callback]
    fn on_tab_view_create_window(&self, _tab_view: &adw::TabView) -> adw::TabView {
        let new_window = FieldMonitorWindow::new(&self.application().unwrap().downcast().unwrap());
        new_window.present();
        new_window.imp().tab_view.clone()
    }

    #[template_callback]
    fn on_overview_open_changed(&self) {
        if self.imp().overview.is_open() {
            self.imp().main_stack.set_visible_child_name("tabs");
        }
    }

    #[template_callback]
    fn on_self_fullscreened_changed(&self) {
        if self.is_fullscreen() {
            self.imp()
                .button_fullscreen
                .set_icon_name("arrows-pointing-inward-symbolic");
        } else {
            self.imp()
                .button_fullscreen
                .set_icon_name("arrows-pointing-outward-symbolic");
        }
    }

    #[template_callback]
    fn on_main_stack_visible_child_name_changed(&self) {
        match self.imp().main_stack.visible_child_name().as_deref() {
            Some("connection-list") => self.set_connection_list_visible(true),
            _ => self.set_connection_list_visible(false),
        }
    }

    #[template_callback]
    fn on_self_connection_list_visible_changed(&self) {
        if self.connection_list_visible() {
            self.add_css_class("connection-list-visible");
            self.remove_css_class("connection-list-not-visible");
        } else {
            self.remove_css_class("connection-list-visible");
            self.add_css_class("connection-list-not-visible");
        }
    }
}

impl FieldMonitorWindow {
    fn act_show_connection_list(&self, _action: &gio::SimpleAction, _param: Option<&Variant>) {
        self.imp()
            .main_stack
            .set_visible_child_name("connection-list");
    }

    fn act_connection_list_toggle_search(
        &self,
        action: &gio::SimpleAction,
        _param: Option<&Variant>,
    ) {
        if let Some(list) = self
            .imp()
            .connection_list_bin
            .child()
            .and_downcast_ref::<FieldMonitorConnectionList>()
        {
            let new_state = !list.search_is_visible();
            list.set_search_is_visible(new_state);
            // state change happens in notify.
        }
    }
}

impl FieldMonitorWindow {
    #[cfg(feature = "devel")]
    fn add_debug_tabs(&self) {
        let debug_widget = gtk::Box::builder()
            .css_classes(["debug-solid-color"])
            .build();
        self.add_new_page(&debug_widget, "Debug 1");
        let debug_widget = gtk::Box::builder()
            .css_classes(["debug-solid-color2"])
            .build();
        self.add_new_page(&debug_widget, "Debug 2");
        self.imp().main_stack.set_visible_child_name("tabs");
    }

    fn add_new_page(&self, page: &impl IsA<gtk::Widget>, title: &str) -> adw::TabPage {
        let tab_page = self.imp().tab_view.append(page);

        tab_page.set_title(title);
        tab_page
    }
}
