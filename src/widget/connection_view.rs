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
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use anyhow::anyhow;
use futures::lock::Mutex;
use gettextrs::gettext;
use glib::object::ObjectExt;
use gtk::gio;
use gtk::glib;
use log::{error, info, warn};
use rdw::DisplayExt;

use libfieldmonitor::adapter::types::AdapterDisplay;
use libfieldmonitor::connection::{ConnectionError, ConnectionResult};

use crate::application::FieldMonitorApplication;
use crate::connection_loader::ConnectionLoader;
use crate::util::configure_vte_styling;
use crate::widget::foucs_grabber::FieldMonitorFocusGrabber;
use crate::widget::window::FieldMonitorWindow;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorConnectionView)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/connection_view.ui")]
    pub struct FieldMonitorConnectionView {
        #[template_child]
        pub outer_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub status_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub error_status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub button_fullscreen: TemplateChild<gtk::Button>,
        #[template_child]
        pub osd_title_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub display_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub header_gradient: TemplateChild<adw::Bin>,
        #[template_child]
        pub focus_grabber: TemplateChild<FieldMonitorFocusGrabber>,
        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        #[property(get, construct_only)]
        pub server_path: RefCell<String>,
        #[property(get, construct_only)]
        pub adapter_id: RefCell<String>,
        #[property(get, set)]
        pub title: RefCell<String>,
        #[property(get, set)]
        pub subtitle: RefCell<String>,
        #[property(get, set, default = true)]
        pub reveal_osd_controls: Cell<bool>,
        #[property(get, set)]
        pub dynamic_resize: Cell<bool>,
        #[property(get, set)]
        pub scale_to_window: Cell<bool>,
        #[property(get, set)]
        pub allow_reauths: Cell<bool>,
        // None: Status not initialized yet
        // true: Connected
        // false: Disconnected
        pub connection_state: RefCell<Option<bool>>,
        pub connection_loader: Mutex<Option<ConnectionLoader>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorConnectionView {
        const NAME: &'static str = "FieldMonitorConnectionView";
        type Type = super::FieldMonitorConnectionView;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);

            klass.install_property_action("view.dynamic-resize", "dynamic-resize");

            klass.install_property_action("view.scale-to-window", "scale-to-window");

            klass.install_action(
                "view.fit-to-screen",
                None,
                |slf: &super::FieldMonitorConnectionView, _, _| {
                    slf.fit_to_screen();
                },
            );
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
    pub fn new(
        app: &FieldMonitorApplication,
        window: Option<&FieldMonitorWindow>,
        server_path: &str,
        adapter_id: &str,
        loader: ConnectionLoader,
    ) -> Self {
        let slf: Self = glib::Object::builder()
            .property("application", app)
            .property("server-path", server_path)
            .property("adapter-id", adapter_id)
            .property("scale-to-window", true)
            .property("reveal-osd-controls", true)
            .property("allow-reauths", true)
            .build();
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

            window.mobile_breakpoint().add_setter(
                &*imp.osd_title_revealer,
                "reveal-child",
                Some(&false.into()),
            );
        }

        imp.connection_loader.try_lock().unwrap().replace(loader);
        glib::spawn_future_local(glib::clone!(
            #[strong]
            slf,
            async move { slf.reset().await }
        ));
        info!("Created connection view for {server_path}");

        slf
    }

    pub async fn reset(&self) {
        info!("Connection view reset");
        let imp = self.imp();

        let mut loader_brw = imp.connection_loader.lock().await;
        let loader = loader_brw.as_mut().unwrap();
        imp.connection_state.replace(None);

        let adapter_id = { imp.adapter_id.borrow().clone() };
        let Some(adapter) = loader
            .create_adapter(&adapter_id, self.allow_reauths())
            .await
        else {
            // we disallow reauth because the adapter creator already tries that. it also already
            // shows a detailed error message, so we don't need to.
            self.handle_error(
                Err(ConnectionError::General(
                    None,
                    anyhow!("Failed to create adapter"),
                )),
                false,
            );
            return;
        };

        let display = adapter.create_and_connect_display(
            Rc::new(glib::clone!(
                #[strong(rename_to = slf)]
                self,
                move || slf.on_connected()
            )),
            Rc::new(glib::clone!(
                #[strong(rename_to = slf)]
                self,
                move |result| slf.on_disconnected(result)
            )),
        );

        self.add_display(display);
    }

    pub fn add_display(&self, display: AdapterDisplay) {
        let imp = self.imp();
        let widget: gtk::Widget = match display {
            AdapterDisplay::Rdw(display) => {
                imp.header_gradient.set_visible(true);
                display.set_visible(true);
                display.set_vexpand(true);
                display.set_hexpand(true);
                self.configure_rdw_action_support(Some(&display));
                imp.focus_grabber.set_display(Some(&display));
                display.upcast()
            }
            AdapterDisplay::Vte(terminal) => {
                imp.header_gradient.set_visible(false);
                terminal.set_vexpand(true);
                terminal.set_hexpand(true);

                // Add a visual black bar to the top, see status stack
                let bx = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .css_classes(["vte-box"])
                    .spacing(12)
                    .build();

                bx.append(
                    &gtk::WindowHandle::builder()
                        .hexpand(true)
                        .vexpand(false)
                        .height_request(46)
                        .css_classes(["faux-header", "vte"])
                        .build(),
                );

                // make vte react to theme
                let style_manager = self.application().unwrap().style_manager();
                style_manager.connect_dark_notify(glib::clone!(
                    #[weak]
                    terminal,
                    move |style_manager| configure_vte_styling(&terminal, style_manager)
                ));
                configure_vte_styling(&terminal, &style_manager);

                bx.append(&terminal);

                self.configure_rdw_action_support(None);
                imp.focus_grabber.set_display(None::<rdw::Display>);
                bx.upcast()
            }
            AdapterDisplay::Arbitrary { widget, overlayed } => {
                let bx = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .build();

                if !overlayed {
                    imp.header_gradient.set_visible(false);
                    bx.append(
                        &gtk::WindowHandle::builder()
                            .hexpand(true)
                            .vexpand(false)
                            .height_request(46)
                            .css_classes(["faux-header"])
                            .build(),
                    )
                } else {
                    imp.header_gradient.set_visible(true);
                }

                bx.append(&widget);

                self.configure_rdw_action_support(None);
                imp.focus_grabber.set_display(None::<rdw::Display>);
                bx.upcast()
            }
        };

        imp.display_bin.set_child(Some(&widget));
    }

    pub fn on_connected(&self) {
        let imp = self.imp();
        let state = &mut *imp.connection_state.borrow_mut();
        match state {
            None => {
                info!("Connection connected.");
                *state = Some(true)
            }
            Some(true) => {
                warn!("Got multiple on_connected events. Ignoring.");
                return;
            }
            Some(false) => {
                warn!("Connection reconnected.");
                *state = Some(true);
            }
        }
        imp.outer_stack.set_visible_child_name("connection");
        self.fit_to_screen();
    }

    pub fn on_disconnected(&self, result: ConnectionResult<()>) {
        let imp = self.imp();
        let state = &mut *imp.connection_state.borrow_mut();
        match state {
            None => {
                info!("Connection failed to establish.");
                *state = Some(false)
            }
            Some(true) => {
                info!("Connection got disconnected.");
                *state = Some(false)
            }
            Some(false) => {
                warn!("Got multiple on_disconnected events. Ignoring.");
                return;
            }
        }

        self.handle_error(result, true)
    }

    fn handle_error(&self, result: ConnectionResult<()>, allow_reauth: bool) {
        let imp = self.imp();

        match result {
            Ok(()) => {
                imp.status_stack.set_visible_child_name("disconnected");
                imp.outer_stack.set_visible_child_name("status");
                self.remove_css_class("connection-view-grabbed");

                imp.error_status_page.set_title(&gettext("Disconnected"));
                imp.error_status_page
                    .set_description(Some(&gettext("The connection to the server was closed.")));
            }
            Err(ConnectionError::AuthFailed(_msg, err)) if allow_reauth && self.allow_reauths() => {
                warn!("Connection failed with auth error: {err}");
                glib::spawn_future_local(glib::clone!(
                    #[strong(rename_to = slf)]
                    self,
                    async move {
                        let mut loader_brw = slf.imp().connection_loader.lock().await;
                        match loader_brw.as_mut().unwrap().reauth().await {
                            Some(()) => {
                                drop(loader_brw);
                                slf.set_allow_reauths(false);
                                slf.reset().await
                            }
                            None => slf.handle_error(
                                Err(ConnectionError::General(
                                    None,
                                    anyhow!("Failed to authenticate"),
                                )),
                                false,
                            ),
                        };
                    }
                ));
            }
            Err(ConnectionError::General(msg, err))
            | Err(ConnectionError::AuthFailed(msg, err)) => {
                imp.status_stack.set_visible_child_name("disconnected");
                imp.outer_stack.set_visible_child_name("status");
                self.remove_css_class("connection-view-grabbed");

                warn!("Connection failed: {err}");
                imp.error_status_page
                    .set_title(&gettext("Connection Failed"));
                let base_desc = gettext("The connection was closed due to an error.");
                let desc = match msg {
                    None => base_desc,
                    Some(msg) => format!("{base_desc}\n{msg}"),
                };
                imp.error_status_page.set_description(Some(&desc))
            }
        }
    }

    fn configure_rdw_action_support(&self, display: Option<&rdw::Display>) {
        match display {
            None => {
                self.action_set_enabled("view.dynamic-resize", false);
                self.action_set_enabled("view.scale-to-window", false);
                self.action_set_enabled("view.fit-to-screen", false);
            }
            Some(_) => {
                self.action_set_enabled("view.dynamic-resize", false); // TODO: Not implemented in rdw yet?
                self.action_set_enabled("view.scale-to-window", true);
                self.action_set_enabled("view.fit-to-screen", true);
            }
        }
        self.notify_dynamic_resize();
        self.notify_scale_to_window();
    }

    fn fit_to_screen(&self) {
        let display = self
            .imp()
            .display_bin
            .child()
            .map(Cast::downcast::<rdw::Display>)
            .and_then(Result::ok);
        let window = self
            .root()
            .map(Cast::downcast::<FieldMonitorWindow>)
            .and_then(Result::ok);

        if let (Some(display), Some(window)) = (display, window) {
            if let Some((w, h)) = display.display_size() {
                if w != 0 && h != 0 {
                    window.resize(w, h);
                }
            }
        }
    }
}

#[gtk::template_callbacks]
impl FieldMonitorConnectionView {
    #[template_callback]
    fn on_self_dynamic_resize_changed(&self) {
        error!("Dynamic resize not implemented");
    }

    #[template_callback]
    fn on_self_scale_to_window_changed(&self) {
        let display = self
            .imp()
            .display_bin
            .child()
            .map(Cast::downcast::<rdw::Display>)
            .and_then(Result::ok);
        if let Some(display) = display {
            if self.scale_to_window() {
                display.set_hexpand(true);
                display.set_vexpand(true);
                display.set_halign(gtk::Align::Fill);
                display.set_valign(gtk::Align::Fill);
            } else {
                display.set_hexpand(false);
                display.set_vexpand(false);
                display.set_halign(gtk::Align::Center);
                display.set_valign(gtk::Align::Center);
            }
        }
    }

    #[template_callback]
    fn on_focus_grabber_grabbed_changed(&self) {
        let grabber = &*self.imp().focus_grabber;
        let grabbed = grabber.grabbed();

        let window = self
            .root()
            .map(Cast::downcast::<FieldMonitorWindow>)
            .and_then(Result::ok);

        self.set_reveal_osd_controls(!grabbed);

        if let Some(window) = window {
            if grabbed {
                window.add_css_class("connection-view-grabbed");
            } else {
                window.remove_css_class("connection-view-grabbed");
            }
        }
    }
}
