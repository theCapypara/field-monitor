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

use crate::application::FieldMonitorApplication;
use crate::remote_server_info::RemoteServerInfo;
use crate::settings::{SettingHeaderBarBehavior, SettingSharpWindowCorners};
use crate::widget::close_warning_dialog::FieldMonitorCloseWarningDialog;
use crate::widget::connection_list::{
    FieldMonitorConnectionStack, FieldMonitorNavbarConnectionList,
};
use crate::widget::connection_view::{
    FieldMonitorConnectionTabView, FieldMonitorNavbarConnectionView, FieldMonitorServerScreen,
};
use crate::widget::quick_connect_dialog::FieldMonitorQuickConnectDialog;
use adw::prelude::*;
use adw::subclass::prelude::*;
use async_std::task::sleep;
use gettextrs::gettext;
use gtk::{gdk, gio, glib};
use log::debug;
use std::cell::Cell;
use std::cell::RefCell;
use std::time::Duration;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/window.ui")]
    pub struct FieldMonitorWindow {
        #[template_child]
        pub outer_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub inner_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub main_split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub connection_view_split_view: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        pub layout_view: TemplateChild<adw::MultiLayoutView>,
        #[template_child]
        pub inner_list_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub mobile_breakpoint: TemplateChild<adw::Breakpoint>,
        #[template_child]
        pub button_fullscreen: TemplateChild<gtk::Button>,
        #[template_child]
        pub welcome_status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub connection_list_stack: TemplateChild<FieldMonitorConnectionStack>,
        #[template_child]
        pub active_connection_tab_view: TemplateChild<FieldMonitorConnectionTabView>,
        #[template_child]
        pub navbar_connection_view: TemplateChild<FieldMonitorNavbarConnectionView>,
        #[template_child]
        pub navbar_connection_list: TemplateChild<FieldMonitorNavbarConnectionList>,
        #[template_child]
        pub connection_list_navbar_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub welcome_button_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub welcome_window_title: TemplateChild<adw::WindowTitle>,
        pub tab_title_notify_binding: RefCell<Option<(gtk::Widget, glib::SignalHandlerId)>>,
        pub force_close: Cell<bool>,
        pub inhibit_possible_sidebar_click: Cell<bool>,
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
        {
            slf.add_css_class("devel");
            slf.imp()
                .welcome_status_page
                .set_icon_name(Some("de.capypara.FieldMonitor.Devel"));
        }

        application.connect_notify_local(
            Some("busy"),
            glib::clone!(
                #[weak]
                slf,
                move |app, _| {
                    if app.busy() {
                        slf.set_cursor(gdk::Cursor::from_name("wait", None).as_ref());
                    } else if !app.loading_connections() {
                        slf.set_cursor(None);
                    }
                }
            ),
        );

        application.connect_notify_local(
            Some("loading-connections"),
            glib::clone!(
                #[weak]
                slf,
                move |app, _| slf.on_app_loading_connections_changed(app)
            ),
        );

        application.connect_local(
            "connection-updated",
            false,
            glib::clone!(
                #[weak]
                slf,
                #[upgrade_or_default]
                move |_| {
                    // If a connection was updated then we now no longer have 0 connections if we
                    // did before, so in that case switch to the normal app mode
                    slf.maybe_disable_no_sidebar_mode();
                    None
                }
            ),
        );

        application.connect_notify_local(
            Some("starting"),
            glib::clone!(
                #[weak]
                slf,
                move |app, _| slf.on_app_starting_changed(app)
            ),
        );

        slf.on_app_loading_connections_changed(application);
        slf.on_app_starting_changed(application);

        if let Some(settings) = application.settings() {
            slf.on_settings_sharp_window_corners_changed(settings.sharp_window_corners());
            slf.on_settings_header_bar_behavior_changed(settings.header_bar_behavior());
            settings.connect_sharp_window_corners_notify(glib::clone!(
                #[weak]
                slf,
                move |settings| slf
                    .on_settings_sharp_window_corners_changed(settings.sharp_window_corners())
            ));
            settings.connect_header_bar_behavior_notify(glib::clone!(
                #[weak]
                slf,
                move |settings| slf
                    .on_settings_header_bar_behavior_changed(settings.header_bar_behavior())
            ));
        } else {
            slf.on_settings_sharp_window_corners_changed(Default::default());
            slf.on_settings_header_bar_behavior_changed(Default::default());
        }

        slf
    }

    fn setup_actions(&self) {
        self.add_action(&gio::PropertyAction::new(
            "fullscreen",
            self,
            "fullscreened",
        ));
        self.add_action_entries([
            gio::ActionEntry::builder("open-quick-connect")
                .activate(glib::clone!(
                    #[weak(rename_to=slf)]
                    self,
                    move |_, _, _| {
                        let dialog = FieldMonitorQuickConnectDialog::new(
                            &slf.application().and_downcast().unwrap(),
                            &slf,
                        );
                        dialog.present(Some(&slf));
                    }
                ))
                .build(),
            gio::ActionEntry::builder("show-sidebar")
                .activate(glib::clone!(
                    #[weak(rename_to=slf)]
                    self,
                    move |_, _, _| {
                        if slf.imp().layout_view.layout_name().as_deref() == Some("connection-view")
                        {
                            slf.imp().connection_view_split_view.set_show_sidebar(true);
                        }
                    }
                ))
                .build(),
        ]);
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

    /// Try to focus an already open connection view, if a connection view for the given
    /// server is open
    pub fn focus_connection_view(&self, server_path: &str, adapter_id: &str) -> bool {
        if self
            .imp()
            .active_connection_tab_view
            .focus(server_path, adapter_id)
        {
            self.imp().layout_view.set_layout_name("connection-view");
            true
        } else {
            false
        }
    }

    /// Open a connection view
    pub fn open_connection_view(&self, info: RemoteServerInfo) {
        self.imp().active_connection_tab_view.open(self, info);
        self.select_connection_view();
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

    fn change_window_title(&self, title: WindowTitle) {
        let imp = self.imp();
        let field_monitor_str = gettext("Field Monitor");
        match title {
            WindowTitle::Main => {
                if let Some((tab, signal)) = imp.tab_title_notify_binding.borrow_mut().take() {
                    glib::signal_handler_disconnect(&tab, signal);
                }
                self.set_title(Some(&field_monitor_str));
            }
            WindowTitle::ConnectionView(tab) => {
                fn change_title(
                    slf: &FieldMonitorWindow,
                    tab: &FieldMonitorServerScreen,
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

    pub(crate) fn tab_view(&self) -> FieldMonitorConnectionTabView {
        self.imp().active_connection_tab_view.get()
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
        } else if imp.active_connection_tab_view.n_pages() > 0 {
            // Handle still open connections and ask user to confirm.

            let open_connection_descs = imp.active_connection_tab_view.describe_active();

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
    fn on_layout_view_layout_name_changed(&self) {
        if let Some("connection-view") = self.imp().layout_view.layout_name().as_deref() {
            if let Some(view) = self.imp().active_connection_tab_view.current() {
                self.change_window_title(WindowTitle::ConnectionView(view));
            }
            self.add_css_class("connection-view-active");
        } else {
            self.change_window_title(WindowTitle::Main);
            self.remove_css_class("connection-view-active");
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
    fn on_inner_list_stack_visible_child_name_changed(&self) {
        let imp = self.imp();
        if imp.inner_list_stack.visible_child_name().as_deref() != Some("connection-list") {
            imp.connection_list_stack.unselect_connection();
        }
    }

    #[template_callback]
    fn on_connection_list_visible_connection_id_changed(&self) {
        let imp = self.imp();
        let connection_id = imp.connection_list_stack.visible_connection_id();
        debug!(
            "Connection List page changed: {:?} - inner list stack page: {:?}",
            connection_id,
            imp.inner_list_stack.visible_child_name()
        );

        let connection_list_visible =
            imp.inner_list_stack.visible_child_name().as_deref() == Some("connection-list");

        if connection_id.is_none() && connection_list_visible {
            debug!("switching to welcome view");
            imp.inner_list_stack.set_visible_child_name("welcome");
        } else if connection_id.is_some() {
            if !connection_list_visible {
                debug!("switching to connection list");
                self.unselect_connection_view();
                imp.inner_list_stack
                    .set_visible_child_name("connection-list");
                self.maybe_disable_no_sidebar_mode();
            }
            self.maybe_clicked_item_on_sidebar();
        }
    }

    #[template_callback]
    fn on_active_connection_tab_view_visible_page_changed(&self) {
        let imp = self.imp();
        let page = imp.active_connection_tab_view.visible_page();
        debug!(
            "Connection View active page changed: {:?} - inner stack page: {:?}",
            page,
            imp.inner_stack.visible_child_name()
        );

        let connection_view_visible =
            imp.inner_stack.visible_child_name().as_deref() == Some("connection-view");

        if page.is_none() && connection_view_visible {
            debug!("switching to welcome view");
            self.unselect_connection_view();
            imp.inner_list_stack.set_visible_child_name("welcome");
        } else if page.is_some() {
            if !connection_view_visible {
                debug!("switching to connection view");
                self.select_connection_view();
            }
            self.maybe_clicked_item_on_sidebar();
        }
    }

    /// called when the window is small and the user hits the button to return to the "sidebar page".
    #[template_callback]
    fn on_main_split_view_show_content_changed(&self) {
        let imp = self.imp();
        if !imp.main_split_view.shows_content() {
            // unselect all items in the sidebar for cosmetic clarity. We do this with some delay
            // so it doesn't happen during the transition animation.
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to=slf)]
                self,
                async move {
                    sleep(Duration::from_millis(100)).await;
                    let imp = slf.imp();
                    // just in case the user resizes the window:
                    imp.inner_list_stack.set_visible_child_name("welcome");
                    imp.inhibit_possible_sidebar_click.set(true);
                    slf.unselect_connection_view();
                    slf.unselect_connection_list();
                    imp.inhibit_possible_sidebar_click.set(false);
                }
            ));
        }
    }

    fn on_app_loading_connections_changed(&self, app: &FieldMonitorApplication) {
        if app.loading_connections() {
            self.imp()
                .connection_list_navbar_stack
                .set_visible_child_name("loader");
        } else {
            self.imp()
                .connection_list_navbar_stack
                .set_visible_child_name("list");

            if self.imp().connection_list_stack.is_empty() {
                if self.imp().layout_view.layout_name().as_deref() == Some("main") {
                    // Enable "no sidebar mode"
                    self.imp()
                        .inner_list_stack
                        .set_visible_child_name("welcome");
                    self.imp().welcome_button_box.set_visible(true);
                    self.imp().layout_view.set_layout_name("no-sidebar");
                    self.imp()
                        .welcome_status_page
                        .set_description(Some(&gettext(
                            "Connect to your virtual machines and remote servers.",
                        )));
                }
            } else {
                self.maybe_disable_no_sidebar_mode();
            }
        }
    }

    fn on_app_starting_changed(&self, app: &FieldMonitorApplication) {
        if app.starting() {
            debug!("app is starting");
            self.imp().outer_stack.set_visible_child_name("starting");
        } else {
            debug!("app is not starting");
            self.imp().outer_stack.set_visible_child_name("app");
        }
    }

    fn on_settings_sharp_window_corners_changed(&self, value: SettingSharpWindowCorners) {
        match value {
            SettingSharpWindowCorners::Auto => {
                self.add_css_class("use-sharp-corners");
                self.remove_css_class("always-sharp");
            }
            SettingSharpWindowCorners::Always => {
                self.add_css_class("use-sharp-corners");
                self.add_css_class("always-sharp");
            }
            SettingSharpWindowCorners::Never => {
                self.remove_css_class("use-sharp-corners");
                self.remove_css_class("always-sharp");
            }
        }
    }

    fn on_settings_header_bar_behavior_changed(&self, value: SettingHeaderBarBehavior) {
        if matches!(value, SettingHeaderBarBehavior::Overlay) {
            self.add_css_class("overlay-headerbar");
        } else {
            self.remove_css_class("overlay-headerbar");
        }
    }

    pub(crate) fn select_connection_view(&self) {
        self.unselect_connection_list();
        self.imp()
            .inner_stack
            .set_visible_child_name("connection-view");
        self.imp().layout_view.set_layout_name("connection-view");
    }

    fn unselect_connection_view(&self) {
        let imp = self.imp();
        if imp.inner_stack.visible_child_name().as_deref() == Some("connection-view") {
            imp.active_connection_tab_view
                .set_visible_page(None::<&adw::TabPage>);
            imp.inner_stack.set_visible_child_name("main");
            imp.layout_view.set_layout_name("main");
        }
    }

    fn unselect_connection_list(&self) {
        self.imp().connection_list_stack.unselect_connection();
    }

    fn maybe_clicked_item_on_sidebar(&self) {
        let imp = self.imp();
        if !imp.inhibit_possible_sidebar_click.get() {
            imp.main_split_view.set_show_content(true);
            imp.connection_view_split_view.set_show_sidebar(false);
        }
    }

    /// Disable the no-sidebar mode used for initial presentation if no connections are present.
    fn maybe_disable_no_sidebar_mode(&self) {
        self.imp().welcome_button_box.set_visible(false);
        self.imp().welcome_status_page.set_description(None);
        if self.imp().layout_view.layout_name().as_deref() == Some("no-sidebar") {
            self.imp().layout_view.set_layout_name("main");
        }
    }
}

#[derive(Debug, Clone)]
enum WindowTitle {
    Main,
    ConnectionView(FieldMonitorServerScreen),
}
