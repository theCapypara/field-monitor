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
use std::borrow::Cow;
use std::cell::Cell;
use std::cell::RefCell;
use std::iter;
use std::rc::Rc;

use adw::gdk::{Key, ModifierType};
use adw::prelude::*;
use adw::subclass::prelude::*;
use anyhow::anyhow;
use futures::lock::Mutex;
use gettextrs::gettext;
use glib::object::ObjectExt;
use gtk::gio;
use gtk::glib;
use log::{debug, info, warn};
use rdw::DisplayExt;
use vte::TerminalExt;

use libfieldmonitor::adapter::types::{AdapterDisplay, AdapterDisplayWidget};
use libfieldmonitor::connection::{ConnectionError, ConnectionResult};
use libfieldmonitor::i18n::gettext_f;

use crate::application::FieldMonitorApplication;
use crate::connection_loader::ConnectionLoader;
use crate::util::configure_vte_styling;
use crate::widget::foucs_grabber::FieldMonitorFocusGrabber;
use crate::widget::grab_note::FieldMonitorGrabNote;
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
        #[template_child]
        pub grab_note: TemplateChild<FieldMonitorGrabNote>,
        #[template_child]
        pub menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub show_output_button: TemplateChild<gtk::Button>,
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
        pub adapter: RefCell<Option<Box<dyn AdapterDisplay>>>,
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

            klass.install_action(
                "view.reconnect",
                None,
                |slf: &super::FieldMonitorConnectionView, _, _| {
                    glib::spawn_future_local(glib::clone!(
                        #[strong]
                        slf,
                        async move { slf.reset().await }
                    ));
                },
            );

            klass.install_action(
                "view.send-keys",
                Some(&String::static_variant_type()),
                |slf: &super::FieldMonitorConnectionView, _, params| {
                    debug!("view.send-keys: {params:?}");
                    let Some(keys) = params.and_then(String::from_variant) else {
                        return;
                    };
                    slf.send_keys(&keys);
                },
            );

            // Show the VTE output after the connection has been disconnected.
            klass.install_action(
                "view.show-output",
                None,
                |slf: &super::FieldMonitorConnectionView, _, _| {
                    debug!("view.show-output");
                    slf.imp().outer_stack.set_visible_child_name("connection");
                },
            );

            klass.install_action(
                "view.term-copy",
                None,
                |slf: &super::FieldMonitorConnectionView, _, _| {
                    debug!("view.term-copy");
                    slf.send_term_command(TermCommand::Copy);
                },
            );

            klass.install_action(
                "view.term-paste",
                None,
                |slf: &super::FieldMonitorConnectionView, _, _| {
                    debug!("view.term-paste");
                    slf.send_term_command(TermCommand::Paste);
                },
            );

            klass.install_action(
                "view.term-select-all",
                None,
                |slf: &super::FieldMonitorConnectionView, _, _| {
                    debug!("view.term-select-all");
                    slf.send_term_command(TermCommand::SelectAll);
                },
            );

            klass.install_action(
                "view.term-zoom-reset",
                None,
                |slf: &super::FieldMonitorConnectionView, _, _| {
                    debug!("view.term-zoom-reset");
                    slf.send_term_command(TermCommand::ZoomReset);
                },
            );

            klass.install_action(
                "view.term-zoom-in",
                None,
                |slf: &super::FieldMonitorConnectionView, _, _| {
                    debug!("view.term-zoom-in");
                    slf.send_term_command(TermCommand::ZoomIn);
                },
            );

            klass.install_action(
                "view.term-zoom-out",
                None,
                |slf: &super::FieldMonitorConnectionView, _, _| {
                    debug!("view.term-zoom-out");
                    slf.send_term_command(TermCommand::ZoomOut);
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

    impl Drop for FieldMonitorConnectionView {
        fn drop(&mut self) {
            debug!("drop FieldMonitorConnectionView");
        }
    }
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
            .property("dynamic-resize", true)
            .property("scale-to-window", true)
            .property("reveal-osd-controls", true)
            .property("allow-reauths", true)
            .build();
        let imp = slf.imp();

        slf.add_menu(MenuKind::Other, vec![]);

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

    pub fn is_connected(&self) -> bool {
        self.imp().connection_state.borrow().unwrap_or_default()
    }

    pub async fn reset(&self) {
        info!("Connection view reset");
        let imp = self.imp();
        imp.status_stack.set_visible_child_name("loading");
        imp.outer_stack.set_visible_child_name("status");

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
                #[weak(rename_to = slf)]
                self,
                move || slf.on_connected()
            )),
            Rc::new(glib::clone!(
                #[weak(rename_to = slf)]
                self,
                move |result| slf.on_disconnected(result)
            )),
        );

        let actions = loader.actions();

        self.add_display(display, actions);
    }

    pub fn send_keys(&self, keys: &str) {
        let acc = gtk::accelerator_parse(keys);
        debug!("parsed keys: {acc:?}");
        if let Some((key, mods)) = acc {
            let keys = mods
                .iter()
                .filter_map(|modf| {
                    if modf == ModifierType::SHIFT_MASK {
                        Some(Key::Shift_L)
                    } else if modf == ModifierType::LOCK_MASK {
                        Some(Key::Caps_Lock)
                    } else if modf == ModifierType::CONTROL_MASK {
                        Some(Key::Control_L)
                    } else if modf == ModifierType::ALT_MASK {
                        Some(Key::Alt_L)
                    } else if modf == ModifierType::SUPER_MASK {
                        Some(Key::Super_L)
                    } else if modf == ModifierType::HYPER_MASK {
                        Some(Key::Hyper_L)
                    } else if modf == ModifierType::META_MASK {
                        Some(Key::Meta_L)
                    } else {
                        None
                    }
                })
                .chain(iter::once(key))
                .collect::<Vec<_>>();
            debug!("processed keys: {keys:?}");
            let display = self
                .imp()
                .display_bin
                .child()
                .map(Cast::downcast::<rdw::Display>)
                .and_then(Result::ok);
            if let Some(display) = display {
                display.send_keys(&keys);
            }
        }
    }

    fn send_term_command(&self, cmd: TermCommand) {
        let brw = self.imp().adapter.borrow();
        if let Some(AdapterDisplayWidget::Vte(vte)) = brw.as_ref().map(|adapter| adapter.widget()) {
            match cmd {
                TermCommand::Copy => vte.copy_clipboard_format(vte::Format::Text),
                TermCommand::Paste => vte.paste_clipboard(),
                TermCommand::SelectAll => vte.select_all(),
                TermCommand::ZoomReset => vte.set_font_scale(1.0),
                TermCommand::ZoomIn => vte.set_font_scale(vte.font_scale() + 0.1),
                TermCommand::ZoomOut => vte.set_font_scale(vte.font_scale() - 0.1),
            }
        }
    }

    pub fn add_display(
        &self,
        display: Box<dyn AdapterDisplay>,
        server_actions: Vec<(Cow<str>, Cow<str>)>,
    ) {
        let imp = self.imp();
        let display_widget = display.widget();

        let widget: gtk::Widget = match &display_widget {
            AdapterDisplayWidget::Rdw(display) => {
                imp.header_gradient.set_visible(true);
                display.set_visible(true);
                display.set_vexpand(true);
                display.set_hexpand(true);
                imp.focus_grabber.set_display(Some(display));
                self.add_menu(MenuKind::Rdw, server_actions);
                display.clone().upcast()
            }
            AdapterDisplayWidget::Vte(terminal) => {
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
                configure_vte_styling(terminal, &style_manager);

                bx.append(terminal);

                self.setup_vte_event_controllers(terminal);
                self.setup_vte_menu_model(terminal);
                imp.focus_grabber.set_display(None);
                self.add_menu(MenuKind::Vte, server_actions);
                bx.upcast()
            }
            AdapterDisplayWidget::Arbitrary { widget, overlayed } => {
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

                bx.append(widget);

                imp.focus_grabber.set_display(None);
                self.add_menu(MenuKind::Other, server_actions);
                bx.upcast()
            }
        };

        self.configure_rdw_action_support(&display_widget);

        imp.adapter.borrow_mut().replace(display);
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

        self.add_menu(MenuKind::Other, vec![]);

        match result {
            Ok(()) => {
                imp.status_stack.set_visible_child_name("disconnected");
                imp.outer_stack.set_visible_child_name("status");
                self.mark_as_ungrabbed();

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
                self.mark_as_ungrabbed();

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

    fn configure_rdw_action_support(&self, display: &AdapterDisplayWidget) {
        let mut is_rdw = false;
        let mut is_vte = false;

        match display {
            AdapterDisplayWidget::Rdw(_) => is_rdw = true,
            AdapterDisplayWidget::Vte(_) => is_vte = true,
            AdapterDisplayWidget::Arbitrary { .. } => {}
        }

        self.action_set_enabled("view.dynamic-resize", is_rdw);
        self.action_set_enabled("view.scale-to-window", is_rdw);
        self.action_set_enabled("view.fit-to-screen", is_rdw);
        if !is_rdw {
            self.imp().dynamic_resize.set(false);
        }

        // Configure the "Show Output" button for disconnected connections.
        self.imp().show_output_button.set_visible(is_vte);
        self.action_set_enabled("view.show-output", is_vte);
        self.action_set_enabled("view.term-copy", is_vte);
        self.action_set_enabled("view.term-paste", is_vte);
        self.action_set_enabled("view.term-select-all", is_vte);
        self.action_set_enabled("view.term-zoom-reset", is_vte);
        self.action_set_enabled("view.term-zoom-in", is_vte);
        self.action_set_enabled("view.term-zoom-out", is_vte);

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

    fn mark_as_ungrabbed(&self) {
        let window = self
            .root()
            .map(Cast::downcast::<FieldMonitorWindow>)
            .and_then(Result::ok);

        if let Some(window) = window {
            window.remove_css_class("connection-view-grabbed");
        }
    }

    fn setup_vte_event_controllers(&self, terminal: &vte::Terminal) {
        let shortcut_controller = gtk::ShortcutController::new();
        shortcut_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        shortcut_controller.add_shortcut(
            gtk::Shortcut::builder()
                .trigger(&gtk::ShortcutTrigger::parse_string("<Shift><Primary>C").unwrap())
                .action(&gtk::ShortcutAction::parse_string("action(view.term-copy)").unwrap())
                .build(),
        );
        shortcut_controller.add_shortcut(
            gtk::Shortcut::builder()
                .trigger(&gtk::ShortcutTrigger::parse_string("<Shift><Primary>V").unwrap())
                .action(&gtk::ShortcutAction::parse_string("action(view.term-paste)").unwrap())
                .build(),
        );
        shortcut_controller.add_shortcut(
            gtk::Shortcut::builder()
                .trigger(&gtk::ShortcutTrigger::parse_string("<Shift><Primary>A").unwrap())
                .action(&gtk::ShortcutAction::parse_string("action(view.term-select-all)").unwrap())
                .build(),
        );
        shortcut_controller.add_shortcut(
            gtk::Shortcut::builder()
                .trigger(&gtk::ShortcutTrigger::parse_string("<Primary>0").unwrap())
                .action(&gtk::ShortcutAction::parse_string("action(view.term-zoom-reset)").unwrap())
                .build(),
        );
        shortcut_controller.add_shortcut(
            gtk::Shortcut::builder()
                .trigger(&gtk::ShortcutTrigger::parse_string("<Primary>plus").unwrap())
                .action(&gtk::ShortcutAction::parse_string("action(view.term-zoom-in)").unwrap())
                .build(),
        );
        shortcut_controller.add_shortcut(
            gtk::Shortcut::builder()
                .trigger(&gtk::ShortcutTrigger::parse_string("<Primary>minus").unwrap())
                .action(&gtk::ShortcutAction::parse_string("action(view.term-zoom-out)").unwrap())
                .build(),
        );

        let scroll_controller = gtk::EventControllerScroll::new(
            gtk::EventControllerScrollFlags::VERTICAL | gtk::EventControllerScrollFlags::DISCRETE,
        );
        scroll_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        scroll_controller.connect_scroll(glib::clone!(
            #[weak_allow_none(rename_to=slf)]
            self,
            move |scroll, _dx, dy| {
                if let Some(slf) = slf {
                    let mods = scroll.current_event_state();

                    if !mods.contains(ModifierType::CONTROL_MASK) || dy == 0.0 {
                        return glib::Propagation::Proceed;
                    }
                    if dy > 0.0 {
                        slf.send_term_command(TermCommand::ZoomOut);
                    } else {
                        slf.send_term_command(TermCommand::ZoomIn);
                    }

                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));

        terminal.add_controller(shortcut_controller);
        terminal.add_controller(scroll_controller);
    }

    fn setup_vte_menu_model(&self, terminal: &vte::Terminal) {
        let menu = Self::vte_menu_shortcuts();
        menu.append_section(None, &Self::vte_menu_zoom());
        terminal.set_context_menu_model(Some(&menu));
    }

    fn add_menu(&self, menu_kind: MenuKind, server_actions: Vec<(Cow<str>, Cow<str>)>) {
        let menu = gio::Menu::new();

        match menu_kind {
            MenuKind::Rdw => {
                menu.append_section(
                    None,
                    &build_menu(&[
                        Some(MenuObject::Item(gio::MenuItem::new(
                            Some(&gettext("_Dynamic Resize")),
                            Some("view.dynamic-resize"),
                        ))),
                        Some(MenuObject::Item(gio::MenuItem::new(
                            Some(&gettext("_Scale to Window")),
                            Some("view.scale-to-window"),
                        ))),
                    ]),
                );

                menu.append_section(
                    None,
                    &build_menu(&[
                        Some(MenuObject::Item(gio::MenuItem::new(
                            Some(&gettext("_Resize Window to Screen")),
                            Some("view.fit-to-screen"),
                        ))),
                        Some(MenuObject::Submenu(
                            gettext("Send _Keys"),
                            build_menu(&[
                                Some(MenuObject::Section(build_menu(&[
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+Backspace")),
                                        Some("view.send-keys::<Control><Alt>Backspace"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+Delete")),
                                        Some("view.send-keys::<Control><Alt>Delete"),
                                    ))),
                                ]))),
                                Some(MenuObject::Section(build_menu(&[
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F1")),
                                        Some("view.send-keys::<Control><Alt>F1"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F2")),
                                        Some("view.send-keys::<Control><Alt>F2"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F3")),
                                        Some("view.send-keys::<Control><Alt>F3"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F4")),
                                        Some("view.send-keys::<Control><Alt>F4"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F5")),
                                        Some("view.send-keys::<Control><Alt>F5"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F6")),
                                        Some("view.send-keys::<Control><Alt>F6"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F7")),
                                        Some("view.send-keys::<Control><Alt>F7"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F8")),
                                        Some("view.send-keys::<Control><Alt>F8"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F9")),
                                        Some("view.send-keys::<Control><Alt>F9"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F10")),
                                        Some("view.send-keys::<Control><Alt>F10"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F11")),
                                        Some("view.send-keys::<Control><Alt>F11"),
                                    ))),
                                    Some(MenuObject::Item(gio::MenuItem::new(
                                        Some(&gettext("Ctrl+Alt+F12")),
                                        Some("view.send-keys::<Control><Alt>F12"),
                                    ))),
                                ]))),
                                Some(MenuObject::Section(build_menu(&[Some(MenuObject::Item(
                                    gio::MenuItem::new(
                                        Some(&gettext("Print")),
                                        Some("view.send-keys::Print"),
                                    ),
                                ))]))),
                            ]),
                        )),
                    ]),
                );
            }
            MenuKind::Vte => {
                let menu_vte = Self::vte_menu_shortcuts();
                menu_vte.append_submenu(Some(&gettext("_Zoom")), &Self::vte_menu_zoom());
                menu.append_section(None, &menu_vte);
            }
            _ => {}
        }

        let more_actions = if server_actions.is_empty() {
            None
        } else {
            let server_path = self.server_path();
            let submenu = gio::Menu::new();
            for (action_id, label) in server_actions {
                let action_target = (true, &server_path, &*action_id).to_variant();
                submenu.append_item(&gio::MenuItem::new(
                    Some(&*label),
                    Some(&gio::Action::print_detailed_name(
                        "app.perform-connection-action",
                        Some(&action_target),
                    )),
                ))
            }
            Some(MenuObject::Submenu(gettext("Server _Actions"), submenu))
        };

        menu.append_section(
            None,
            &build_menu(&[
                Some(MenuObject::Item(gio::MenuItem::new(
                    Some(&gettext("_Move to New Window")),
                    Some("tab.move-to-new-window"),
                ))),
                more_actions,
                Some(MenuObject::Item(gio::MenuItem::new(
                    Some(&gettext("_Close Connection")),
                    Some("tab.close"),
                ))),
            ]),
        );

        menu.append_section(
            None,
            &build_menu(&[
                Some(MenuObject::Item(gio::MenuItem::new(
                    Some(&gettext("_New Window")),
                    Some("app.new-window"),
                ))),
                Some(MenuObject::Item(gio::MenuItem::new(
                    Some(&gettext("_Keyboard Shortcuts")),
                    Some("win.show-help-overlay"),
                ))),
                Some(MenuObject::Item(gio::MenuItem::new(
                    Some(&gettext("_About Field Monitor")),
                    Some("app.about"),
                ))),
            ]),
        );

        menu.freeze();
        self.imp().menu_button.set_menu_model(Some(&menu));
    }

    fn vte_menu_shortcuts() -> gio::Menu {
        build_menu(&[
            Some(MenuObject::Item(gio::MenuItem::new(
                Some(&gettext("_Copy")),
                Some("view.term-copy"),
            ))),
            Some(MenuObject::Item(gio::MenuItem::new(
                Some(&gettext("_Paste")),
                Some("view.term-paste"),
            ))),
            Some(MenuObject::Item(gio::MenuItem::new(
                Some(&gettext("Select _All")),
                Some("view.term-select-all"),
            ))),
        ])
    }

    fn vte_menu_zoom() -> gio::Menu {
        build_menu(&[Some(MenuObject::Section(build_menu(&[
            Some(MenuObject::Item(gio::MenuItem::new(
                Some(&gettext("Zoom In")),
                Some("view.term-zoom-in"),
            ))),
            Some(MenuObject::Item(gio::MenuItem::new(
                Some(&gettext("Zoom Out")),
                Some("view.term-zoom-out"),
            ))),
            Some(MenuObject::Item(gio::MenuItem::new(
                Some(&gettext("Reset Zoom")),
                Some("view.term-zoom-reset"),
            ))),
        ])))])
    }
}

#[gtk::template_callbacks]
impl FieldMonitorConnectionView {
    #[template_callback]
    fn on_self_dynamic_resize_changed(&self) {
        let display = self
            .imp()
            .display_bin
            .child()
            .map(Cast::downcast::<rdw::Display>)
            .and_then(Result::ok);

        if let Some(display) = display.as_ref() {
            display.set_remote_resize(self.dynamic_resize());
        }

        // Enable or disable scale to window / fit to screen switches based on if dynamic resize is on.
        if self.dynamic_resize() {
            self.set_scale_to_window(true); // needs also to be on.
            self.action_set_enabled("view.scale-to-window", false);
            //self.action_set_enabled("view.fit-to-screen", false);
        } else if display.is_some() {
            self.action_set_enabled("view.scale-to-window", true);
            //self.action_set_enabled("view.fit-to-screen", true);
        }
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

        if let Some(display) = self
            .imp()
            .display_bin
            .child()
            .and_downcast::<rdw::Display>()
        {
            if grabbed {
                let shortcut = display.grab_shortcut().to_label(&self.display());
                // The shortcut may have alternatives for technical reasons, but only show the
                // first part.
                // TODO: The displayed string is not stable.
                let shortcut = shortcut.split(',').next().unwrap();

                self.imp().grab_note.show_note(&gettext_f(
                    "Press {keycombo} to un-grab the mouse and keyboard.",
                    &[("keycombo", shortcut)],
                ));
            } else {
                self.imp().grab_note.hide_note();
            }
        }
    }

    #[template_callback]
    fn on_self_unrealize(&self) {
        debug!("connection view unrealized");
        self.mark_as_ungrabbed();
    }
}

enum MenuKind {
    Rdw,
    Vte,
    Other,
}

enum MenuObject {
    Item(gio::MenuItem),
    Section(gio::Menu),
    Submenu(String, gio::Menu),
}

fn build_menu(items: &[Option<MenuObject>]) -> gio::Menu {
    let menu = gio::Menu::new();

    for item in items.iter().flatten() {
        match item {
            MenuObject::Item(item) => menu.append_item(item),
            MenuObject::Section(section) => menu.append_section(None, section),
            MenuObject::Submenu(label, submenu) => menu.append_submenu(Some(label), submenu),
        }
    }

    menu
}

enum TermCommand {
    Copy,
    Paste,
    SelectAll,
    ZoomReset,
    ZoomIn,
    ZoomOut,
}
