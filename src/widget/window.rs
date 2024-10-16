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

use std::cell::Cell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use gtk::glib::Variant;
use itertools::Itertools;
use log::debug;

use crate::application::FieldMonitorApplication;
use crate::connection_loader::ConnectionLoader;
use crate::widget::close_warning_dialog::FieldMonitorCloseWarningDialog;
use crate::widget::connection_list::FieldMonitorConnectionList;
use crate::widget::connection_view::FieldMonitorConnectionView;

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
        pub tab_view: TemplateChild<adw::TabView>,
        #[template_child]
        pub overview: TemplateChild<adw::TabOverview>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub mobile_breakpoint: TemplateChild<adw::Breakpoint>,
        pub tab_title_notify_binding: RefCell<Option<(gtk::Widget, glib::SignalHandlerId)>>,
        pub force_close: Cell<bool>,
        // Currently active page for TabView menus.
        pub menu_page: RefCell<Option<adw::TabPage>>,
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
        @implements gtk::Root, gtk::Native, gio::ActionGroup, gio::ActionMap;
}

impl FieldMonitorWindow {
    pub fn new(application: &FieldMonitorApplication) -> Self {
        let slf: Self = glib::Object::builder()
            .property("application", application)
            .build();

        #[cfg(feature = "devel")]
        slf.add_css_class("devel");

        let conn_list = FieldMonitorConnectionList::new(application, Some(&slf));
        slf.imp().connection_list_bin.set_child(Some(&conn_list));
        conn_list.set_show_overview_button(slf.imp().tab_view.n_pages() != 0);
        slf.imp().tab_view.connect_notify_local(
            Some("n-pages"),
            glib::clone!(
                #[weak]
                conn_list,
                move |view, _| conn_list.set_show_overview_button(view.n_pages() != 0)
            ),
        );

        slf.on_main_stack_visible_child_name_changed();

        slf
    }

    fn setup_actions(&self) {
        let show_connection_action = gio::ActionEntry::builder("show-connection-list")
            .activate(Self::act_show_connection_list)
            .build();
        let open_overview_action = gio::ActionEntry::builder("open-overview")
            .activate(Self::act_open_overview)
            .build();

        self.add_action(&gio::PropertyAction::new(
            "fullscreen",
            self,
            "fullscreened",
        ));

        self.add_action_entries([open_overview_action, show_connection_action]);

        let tab_move_to_new_window_action = gio::ActionEntry::builder("move-to-new-window")
            .activate(glib::clone!(
                #[weak(rename_to = slf)]
                self,
                move |_, a, b| slf.act_tab_move_to_new_window(a, b)
            ))
            .build();

        let tab_close_action = gio::ActionEntry::builder("close")
            .activate(glib::clone!(
                #[weak(rename_to = slf)]
                self,
                move |_, a, b| slf.act_tab_close(a, b)
            ))
            .build();

        let tab_action_group = gio::SimpleActionGroup::new();
        tab_action_group.add_action_entries([tab_move_to_new_window_action, tab_close_action]);
        self.insert_action_group("tab", Some(&tab_action_group));
    }

    pub fn toast(&self, msg: &str) {
        self.imp()
            .toast_overlay
            .add_toast(adw::Toast::builder().title(msg).timeout(5).build())
    }

    pub fn toast_overlay(&self) -> &adw::ToastOverlay {
        &self.imp().toast_overlay
    }

    pub fn mobile_breakpoint(&self) -> &adw::Breakpoint {
        &self.imp().mobile_breakpoint
    }

    pub fn show_tabs(&self) {
        self.imp().main_stack.set_visible_child_name("tabs");
    }

    /// Try to focus an already open connection view, if a connection view for the given
    /// server is open
    pub fn focus_connection_view(&self, server_path: &str, adapter_id: &str) -> bool {
        let tab_view = &self.imp().tab_view;
        for page in tab_view.pages().iter::<adw::TabPage>() {
            let Ok(page) = page else {
                return false;
            };
            let Ok(view) = page.child().downcast::<FieldMonitorConnectionView>() else {
                continue;
            };
            if view.server_path() == server_path && view.adapter_id() == adapter_id {
                self.imp().overview.set_open(false);
                tab_view.set_selected_page(&page);
                self.imp().main_stack.set_visible_child_name("tabs");
                return true;
            }
        }
        false
    }

    pub fn open_connection_view(
        &self,
        server_path: &str,
        adapter_id: &str,
        server_title: &str,
        connection_title: &str,
        loader: ConnectionLoader,
    ) {
        let app = self
            .application()
            .unwrap()
            .downcast::<FieldMonitorApplication>()
            .unwrap();

        let view =
            FieldMonitorConnectionView::new(&app, Some(self), server_path, adapter_id, loader);

        self.add_new_page(&view, server_title, Some(connection_title));

        self.imp().main_stack.set_visible_child_name("tabs");
    }

    // Taken in parts from Showtime:
    // https://gitlab.gnome.org/GNOME/Incubator/showtime/-/blob/238f6d37a09fd264b887642f03521d974c369794/showtime/window.py#L836
    pub fn resize(&self, new_width: usize, new_height: usize) {
        debug!("Resizing window");

        let (init_width, init_height) = self.default_size();
        let (init_width, init_height) = (init_width as usize, init_height as usize);

        for (prop, init, target) in [
            ("default-width", init_width, new_width),
            ("default-height", init_height, new_height),
        ] {
            let anim = adw::TimedAnimation::new(
                self,
                init as f64,
                target as f64,
                500,
                adw::PropertyAnimationTarget::new(self, prop),
            );
            anim.set_easing(adw::Easing::EaseOutExpo);
            anim.play();
            debug!("Resized window to {new_width}x{new_height}.")
        }
    }

    fn act_show_connection_list(&self, _action: &gio::SimpleAction, _param: Option<&Variant>) {
        self.imp()
            .main_stack
            .set_visible_child_name("connection-list");
    }

    fn act_open_overview(&self, _action: &gio::SimpleAction, _param: Option<&Variant>) {
        if self.imp().tab_view.n_pages() > 0 {
            self.imp().overview.set_open(true);
            self.imp().main_stack.set_visible_child_name("tabs");
        }
    }

    fn act_tab_move_to_new_window(&self, _action: &gio::SimpleAction, _param: Option<&Variant>) {
        let imp = self.imp();
        if let Some(menu_page) = self.tab_view_current_page() {
            let new_window =
                FieldMonitorWindow::new(&self.application().unwrap().downcast().unwrap());

            let child = menu_page.child();
            new_window.set_default_size(child.width(), child.height());

            imp.tab_view
                .transfer_page(&menu_page, &new_window.imp().tab_view, 0);
            new_window.present();
            new_window.show_tabs();
            if !imp.overview.is_open() {
                imp.main_stack.set_visible_child_name("connection-list");
            }
        }
    }

    fn act_tab_close(&self, _action: &gio::SimpleAction, _param: Option<&Variant>) {
        let imp = self.imp();
        if let Some(menu_page) = self.tab_view_current_page() {
            imp.tab_view.close_page(&menu_page);
            if !imp.overview.is_open() {
                imp.main_stack.set_visible_child_name("connection-list");
            }
        }
    }

    fn tab_view_current_page(&self) -> Option<adw::TabPage> {
        self.imp()
            .menu_page
            .borrow()
            .as_ref()
            .cloned()
            .or_else(|| self.imp().tab_view.selected_page())
    }

    fn add_new_page(
        &self,
        page: &impl IsA<gtk::Widget>,
        title: &str,
        subtitle: Option<&str>,
    ) -> adw::TabPage {
        let tab_view = &self.imp().tab_view;

        let page = page.upcast_ref();
        let tab_page = tab_view.append(page);

        if let Some(view) = page.downcast_ref::<FieldMonitorConnectionView>() {
            view.bind_property("title", &tab_page, "title")
                .bidirectional()
                .build();
            if let Some(subtitle) = subtitle {
                view.set_subtitle(subtitle);
            }
        }

        tab_page.set_title(title);

        self.imp().overview.set_open(false);
        tab_view.set_selected_page(&tab_page);

        tab_page
    }
}

#[gtk::template_callbacks]
impl FieldMonitorWindow {
    #[template_callback]
    fn on_self_close_request(&self) -> bool {
        let imp = self.imp();
        if imp.force_close.get() {
            // User has forced the window to close.

            false
        } else if imp.tab_view.n_pages() > 0 {
            // Handle still open connections and ask user to confirm.

            let open_connection_descs: Vec<_> = imp
                .tab_view
                .pages()
                .iter::<adw::TabPage>()
                .filter_map_ok(|tab| tab.child().downcast::<FieldMonitorConnectionView>().ok())
                .filter_ok(|view| view.is_connected())
                .map_ok(|view| (view.title(), view.subtitle()))
                .collect::<Result<_, _>>()
                .unwrap_or_default();

            if open_connection_descs.is_empty() {
                return false;
            }

            let dialog = FieldMonitorCloseWarningDialog::new(open_connection_descs);

            dialog.connect_closure(
                "response",
                false,
                glib::closure_local!(
                    #[weak(rename_to = slf)]
                    self,
                    move |_: &FieldMonitorCloseWarningDialog, response: &str| {
                        if response == FieldMonitorCloseWarningDialog::RESPONSE_CLOSE {
                            slf.imp().force_close.set(true);
                            slf.close();
                        }
                    }
                ),
            );

            dialog.present(Some(self));

            true
        } else {
            // No open connections, close.

            false
        }
    }

    #[template_callback]
    fn on_tab_view_setup_menu(&self, tab_page: Option<adw::TabPage>, _tab_view: &adw::TabView) {
        self.imp().menu_page.replace(tab_page);
    }

    #[template_callback]
    fn on_tab_view_create_window(&self, _tab_view: &adw::TabView) -> adw::TabView {
        let new_window = FieldMonitorWindow::new(&self.application().unwrap().downcast().unwrap());
        new_window.present();
        new_window.show_tabs();
        new_window.imp().tab_view.clone()
    }

    #[template_callback]
    fn on_overview_open_changed(&self) {
        if self.imp().overview.is_open() {
            self.change_window_title(WindowTitle::Overview);
            self.imp().main_stack.set_visible_child_name("tabs");
        } else {
            self.on_tab_view_selected_page_changed();
        }
    }

    #[template_callback]
    fn on_main_stack_visible_child_name_changed(&self) {
        if let Some("connection-list") = self.imp().main_stack.visible_child_name().as_deref() {
            self.change_window_title(WindowTitle::ConnectionList);
        }
    }

    #[template_callback]
    fn on_tab_view_selected_page_changed(&self) {
        if let Some("tabs") = self.imp().main_stack.visible_child_name().as_deref() {
            if let Some(selected_page) = self.imp().tab_view.selected_page() {
                if let Ok(view) = selected_page
                    .child()
                    .downcast::<FieldMonitorConnectionView>()
                {
                    self.change_window_title(WindowTitle::ConnectionView(view));
                }
            }
        }
    }

    fn change_window_title(&self, title: WindowTitle) {
        let imp = self.imp();
        let field_monitor_str = gettext("Field Monitor");
        match title {
            WindowTitle::Overview | WindowTitle::ConnectionList => {
                if let Some((tab, signal)) = imp.tab_title_notify_binding.borrow_mut().take() {
                    glib::signal_handler_disconnect(&tab, signal);
                }
                self.set_title(Some(&field_monitor_str));
            }
            WindowTitle::ConnectionView(tab) => {
                fn change_title(
                    slf: &FieldMonitorWindow,
                    tab: &FieldMonitorConnectionView,
                    suffix: &str,
                ) {
                    slf.set_title(Some(&format!("{} - {}", tab.title(), suffix)));
                }

                change_title(self, &tab, &field_monitor_str);

                let signal_handler_id = tab.connect_notify_local(
                    Some("title"),
                    glib::clone!(
                        #[weak(rename_to=slf)]
                        self,
                        move |tab, _| {
                            change_title(&slf, tab, &field_monitor_str);
                        }
                    ),
                );

                imp.tab_title_notify_binding
                    .borrow_mut()
                    .replace((tab.upcast(), signal_handler_id));
            }
        }
    }
}

#[derive(Debug, Clone)]
enum WindowTitle {
    Overview,
    ConnectionList,
    ConnectionView(FieldMonitorConnectionView),
}
