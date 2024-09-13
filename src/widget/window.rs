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

use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use gtk::glib::Variant;
use gtk::prelude::*;

use crate::widget::connection_list::FieldMonitorConnectionList;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorWindow)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/window.ui")]
    pub struct FieldMonitorWindow {
        #[template_child]
        pub main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub tab_bar: TemplateChild<adw::TabBar>,
        #[template_child]
        pub tab_view: TemplateChild<adw::TabView>, // all children must be gtk::Box!
        #[template_child]
        pub overview: TemplateChild<adw::TabOverview>,
        #[template_child]
        pub button_connection_list: TemplateChild<gtk::Button>,
        #[template_child]
        pub button_search_in_list: TemplateChild<gtk::Button>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub borderless_box: TemplateChild<gtk::Box>,
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
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        let slf: Self = glib::Object::builder()
            .property("application", application)
            .build();
        #[cfg(feature = "devel")]
        slf.add_css_class("devel");
        slf
    }

    fn setup_actions(&self) {
        let show_connection_action = gio::ActionEntry::builder("show-connection-list")
            .activate(Self::act_show_connection_list)
            .build();

        let toggle_borderless_action = gio::ActionEntry::builder("toggle-borderless")
            .state(false.to_variant())
            .activate(Self::act_toggle_borderless)
            .build();

        let toggle_fullscreen_action = gio::ActionEntry::builder("toggle-fullscreen")
            .state(false.to_variant())
            .activate(Self::act_toggle_fullscreen)
            .build();

        self.add_action_entries([
            show_connection_action,
            toggle_borderless_action,
            toggle_fullscreen_action,
        ]);
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
        let new_window = FieldMonitorWindow::new(&self.application().unwrap());
        new_window.present();
        new_window.imp().tab_view.clone()
    }

    #[template_callback]
    fn on_tab_view_page_attached(&self, _page: &adw::TabPage, _position: i32) {
        if !self.imp().tab_view.is_bound() {
            return;
        }
        self.configure_tab_bar_autohide();
    }

    #[template_callback]
    fn on_tab_view_page_detached(&self, _page: &adw::TabPage, _position: i32) {
        if !self.imp().tab_view.is_bound() {
            return;
        }
        self.configure_tab_bar_autohide();
        if self.imp().tab_view.n_pages() == 0 {
            self.open_new_connection_list();
        }
    }

    /// Hide connection list button if already on connection list.
    #[template_callback]
    fn on_tab_view_notify_selected_page(&self) {
        if !self.imp().tab_view.is_bound() {
            return;
        }
        let selected = self.imp().tab_view.selected_page();
        match selected {
            Some(p) if is_tab_page_connection_list(&p) => {
                self.set_connection_list_visible(true);
            }
            _ => {
                self.set_connection_list_visible(false);
            }
        }
    }

    #[template_callback]
    fn on_button_search_in_list_clicked(&self) {
        let selected = self.imp().tab_view.selected_page();
        if let Some(selected) = selected {
            if let Some(list) = selected
                .child()
                .downcast_ref::<FieldMonitorConnectionList>()
            {
                list.toggle_search();
            }
        }
    }
}

impl FieldMonitorWindow {
    fn act_show_connection_list(&self, _action: &gio::SimpleAction, _param: Option<&Variant>) {
        let page = self
            .get_open_connection_list_page()
            .unwrap_or_else(|| self.open_new_connection_list());
        self.imp().tab_view.set_selected_page(&page);
    }

    fn act_toggle_borderless(&self, action: &gio::SimpleAction, _param: Option<&Variant>) {
        let imp = self.imp();
        let new_state = !action.state().unwrap().get::<bool>().unwrap();

        // Switches the borderless_overlay Bin's child widget with the current tabview Bin's page
        // child widget and then changes the main stack.
        let wdg_a = &*imp.borderless_box;
        let wdg_b = imp
            .tab_view
            .selected_page()
            .and_then(|page| page.child().downcast::<gtk::Box>().ok());
        let Some(wdg_b) = wdg_b else {
            return;
        };
        let (Some(chld_a), Some(chld_b)) = (wdg_a.first_child(), wdg_b.first_child()) else {
            return;
        };
        wdg_a.remove(&chld_a);
        wdg_b.remove(&chld_b);
        wdg_a.append(&chld_b);
        wdg_b.append(&chld_a);

        if new_state {
            imp.main_stack.set_visible_child_name("borderless");
        } else {
            imp.main_stack.set_visible_child_name("normal");
        }

        action.set_state(&new_state.to_variant());
    }

    fn act_toggle_fullscreen(&self, action: &gio::SimpleAction, _param: Option<&Variant>) {
        let new_state = !action.state().unwrap().get::<bool>().unwrap();

        self.set_fullscreened(new_state);

        action.set_state(&new_state.to_variant());
    }
}

impl FieldMonitorWindow {
    fn get_open_connection_list_page(&self) -> Option<adw::TabPage> {
        for child in self.imp().tab_view.pages().iter::<adw::TabPage>() {
            let child = child.unwrap(); // TODO: maybe want to be more graceful? but this really should never happen.
            if is_tab_page_connection_list(&child) {
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

        let page =
            FieldMonitorConnectionList::new(&self.application().unwrap().downcast().unwrap());

        let tab_page = self.add_new_page(&page, &title);

        tab_page.set_live_thumbnail(true);
        tab_page
    }

    fn add_new_page(&self, page: &impl IsA<gtk::Widget>, title: &str) -> adw::TabPage {
        let page = page.upcast_ref();
        page.set_hexpand(true);
        page.set_vexpand(true);

        let boxx = gtk::Box::builder().vexpand(true).hexpand(true).build();
        boxx.append(page);
        let tab_page = self.imp().tab_view.append(&boxx);

        tab_page.set_title(title);
        tab_page
    }
}

fn is_tab_page_connection_list(page: &adw::TabPage) -> bool {
    page.child()
        .downcast_ref::<gtk::Box>()
        .unwrap()
        .first_child()
        .map(|child| {
            child
                .type_()
                .is_a(FieldMonitorConnectionList::static_type())
        })
        .unwrap_or_default()
}
