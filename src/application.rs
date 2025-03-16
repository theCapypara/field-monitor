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
use std::borrow::Cow;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use adw::gio::File;
use adw::glib::user_config_dir;
use adw::prelude::*;
use adw::subclass::prelude::*;
use anyhow::anyhow;
use async_std::fs::{create_dir_all, read_dir, read_to_string, remove_file, OpenOptions};
use async_std::io::WriteExt;
use futures::StreamExt;
use gettextrs::gettext;
use glib::subclass::Signal;
use glib::{ControlFlow, ExitCode, VariantDict};
use gtk::{gio, glib};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use libfieldmonitor::busy::{BusyGuard, BusyStack};
use libfieldmonitor::config::{APP_ID, VERSION};
use libfieldmonitor::connection::ConnectionConfiguration;
use libfieldmonitor::connection::ConnectionInstance;
use libfieldmonitor::connection::ConnectionProvider;
use libfieldmonitor::connection::DualScopedConnectionConfiguration;
use libfieldmonitor::i18n::gettext_f;
use libfieldmonitor::ManagesSecrets;

use crate::connection::CONNECTION_PROVIDERS;
use crate::connection_loader::ConnectionLoader;
use crate::remote_server_info::RemoteServerInfo;
use crate::secrets::SecretManager;
use crate::settings::FieldMonitorSettings;
use crate::widget::add_connection_dialog::FieldMonitorAddConnectionDialog;
use crate::widget::authenticate_connection_dialog::FieldMonitorAuthenticateConnectionDialog;
use crate::widget::preferences::FieldMonitorPreferencesDialog;
use crate::widget::update_connection_dialog::FieldMonitorUpdateConnectionDialog;
use crate::widget::window::FieldMonitorWindow;

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorApplication)]
    pub struct FieldMonitorApplication {
        pub secret_manager: RefCell<Option<Arc<Box<dyn ManagesSecrets>>>>,
        pub connections: RefCell<Option<HashMap<String, ConnectionInstance>>>,
        pub providers: RefCell<HashMap<String, Rc<Box<dyn ConnectionProvider>>>>,
        /// Manages a stack for `pending_server_action`. If stack size is zero, sets to false.
        pub busy_stack: RefCell<Option<BusyStack>>,
        /// Whether Field Monitor is currently loading all connections for the first time.
        #[property(get, construct_only)]
        pub starting: Cell<bool>,
        /// Whether Field Monitor is currently (re-)loading all connections.
        #[property(get)]
        pub loading_connections: Cell<bool>,
        /// Currently busy with processing an action or connection request to a server or connection.
        #[property(get)]
        pub busy: Rc<Cell<bool>>,
        #[property(get, construct_only)]
        pub settings: RefCell<Option<FieldMonitorSettings>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorApplication {
        const NAME: &'static str = "FieldMonitorApplication";
        type Type = super::FieldMonitorApplication;
        type ParentType = adw::Application;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorApplication {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    // This signal is emitted when connections are added or updated.
                    Signal::builder("connection-updated")
                        .param_types([ConnectionInstance::static_type()])
                        .build(),
                    // This signal is emitted when connections are removed.
                    // Listeners should forget the connection with the given ID and drop
                    // all ConnectionConfiguration and ConnectionInstances.
                    Signal::builder("connection-removed")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);

            // Register CLI options
            obj.add_main_option(
                "new-window",
                b'n'.into(),
                glib::OptionFlags::NONE,
                glib::OptionArg::None,
                &gettext("Open the app with a new window"),
                None,
            );

            // Listen to own signals for debug purposes
            self.obj().connect_closure(
                "connection-updated",
                false,
                glib::closure_local!(
                    move |_: super::FieldMonitorApplication, instance: ConnectionInstance| {
                        debug!(
                            "connection updated: tag: {:?}, id: {:?}",
                            instance.provider_tag(),
                            instance.connection_id()
                        );
                    }
                ),
            );
            self.obj().connect_closure(
                "connection-removed",
                false,
                glib::closure_local!(move |_: super::FieldMonitorApplication, id: String| {
                    debug!("connection removed: id: {}", id);
                }),
            );
        }
    }

    impl ApplicationImpl for FieldMonitorApplication {
        fn activate(&self) {
            let obj = self.obj();
            let hold = self.obj().hold();
            glib::spawn_future_local(glib::clone!(
                #[strong]
                obj,
                async move {
                    let slf = obj.imp();
                    let _hold = hold;
                    if let ControlFlow::Continue = slf.init().await {
                        slf.finish_activate(false);
                    }
                }
            ));
        }

        fn open(&self, files: &[File], _hint: &str) {
            if files.is_empty() {
                return self.activate();
            }
            let file = files[0].clone();

            let obj = self.obj();
            let hold = self.obj().hold();
            glib::spawn_future_local(glib::clone!(
                #[strong]
                obj,
                #[strong]
                file,
                async move {
                    let slf = obj.imp();
                    let _hold = hold;
                    if let ControlFlow::Continue = slf.init().await {
                        let force_new_window = obj
                            .settings()
                            .as_ref()
                            .map(FieldMonitorSettings::open_in_new_window)
                            .unwrap_or_default();
                        let window = slf.finish_activate(force_new_window);

                        match RemoteServerInfo::try_from_file(file, &obj, Some(&window)).await {
                            Err(err) => {
                                let alert = adw::AlertDialog::builder()
                                    .heading(gettext("Failed to open"))
                                    .body(format!(
                                        "{}:\n{}",
                                        gettext("Field Monitor could not connect to the server using the specified file or URI"),
                                        err
                                    ))
                                    .build();
                                alert.add_response("ok", &gettext("OK"));
                                alert.present(None::<&gtk::Window>);
                            }
                            Ok(conn) => window.open_connection_view(conn),
                        }
                    }
                }
            ));
        }

        fn startup(&self) {
            self.parent_startup();
            let obj = self.obj();

            obj.imp().busy_stack.borrow_mut().replace(BusyStack::new(
                obj.imp().busy.clone(),
                Box::new(glib::clone!(
                    #[weak]
                    obj,
                    move || obj.notify("busy")
                )),
            ));

            // Accelerators. We remove ALL accelerators first and only use custom accelerators
            // since we remove and re-add them later. Plus some default accelerators are not useful
            // for us, such an unconditional Control+Q to quit.
            // TODO: If somebody knows a more elegant way, let me know.
            obj.remove_accels();
            obj.add_accels();

            // Prefer dark style by default
            obj.style_manager()
                .set_color_scheme(adw::ColorScheme::PreferDark);
        }

        fn handle_local_options(&self, options: &VariantDict) -> ExitCode {
            let obj = self.obj();

            if let Err(err) = obj.register(None::<&gio::Cancellable>) {
                error!("failed to register application: {err}");
                1.into()
            } else if obj.is_remote() {
                if options.contains("new-window") {
                    info!("opening new window");
                    obj.activate_action("new-window", None);
                    0.into()
                } else {
                    info!("focusing current window");
                    (-1).into()
                }
            } else {
                info!("starting Field Monitor...");
                (-1).into()
            }
        }
    }

    impl GtkApplicationImpl for FieldMonitorApplication {}
    impl AdwApplicationImpl for FieldMonitorApplication {}

    impl FieldMonitorApplication {
        async fn init(&self) -> ControlFlow {
            // Init providers if not done already.
            if self.providers.borrow().is_empty() {
                self.providers.replace(
                    CONNECTION_PROVIDERS
                        .iter()
                        .map(|constructor| {
                            let provider = constructor.new();
                            (provider.tag().to_owned(), Rc::new(provider))
                        })
                        .collect(),
                );
            }
            // Init secret service if not done already.
            if self.secret_manager.borrow().is_none() {
                info!("Initializing secrets provider...");
                let slf = self;
                let secrets = SecretManager::new().await;
                match secrets {
                    Ok(secrets) => {
                        slf.secret_manager
                            .replace(Some(Arc::new(Box::new(secrets))));

                        ControlFlow::Continue
                    }
                    Err(err) => {
                        error!("Failed to initialize secrets provider: {err}");
                        let alert = adw::AlertDialog::builder()
                                    .title(gettext("Failed to initialize"))
                                    .body(format!(
                                        "{}:\n{}",
                                        gettext("Field Monitor could not start, because it could not connect to your system's secret service for accessing passwords"),
                                        err
                                    ))
                                    .build();
                        alert.add_response("ok", &gettext("OK"));
                        alert.present(None::<&gtk::Window>);

                        ControlFlow::Break
                    }
                }
            } else {
                ControlFlow::Continue
            }
        }

        fn finish_activate(&self, force_new_window: bool) -> FieldMonitorWindow {
            // Init connections if not done already.
            if self.connections.borrow().is_none() {
                self.connections.borrow_mut().replace(HashMap::new());
                let slf = self;
                glib::spawn_future_local(glib::clone!(
                    #[weak]
                    slf,
                    async move {
                        slf.obj().reload_connections().await;
                    }
                ));
            }

            let application = self.obj();

            // Get the current window or create one if necessary
            let window = if force_new_window {
                FieldMonitorWindow::new(&application)
            } else {
                application
                    .active_window()
                    .and_downcast::<FieldMonitorWindow>()
                    .unwrap_or_else(|| FieldMonitorWindow::new(&application))
            };

            // Ask the window manager/compositor to present the window
            window.present();
            window
        }

        pub fn set_loading_connection(&self, value: bool) {
            self.loading_connections.replace(value);
            self.obj().notify_loading_connections();
            if !value && self.starting.get() {
                self.starting.replace(value);
                self.obj().notify_starting();
            }
        }

        pub fn get_provider(&self, tag: &str) -> Option<Rc<Box<dyn ConnectionProvider>>> {
            self.providers.borrow().get(tag).as_ref().cloned().cloned()
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorApplication(ObjectSubclass<imp::FieldMonitorApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl FieldMonitorApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .property("settings", FieldMonitorSettings::new(APP_ID))
            .property("starting", true)
            .build()
    }

    pub fn remove_accels(&self) {
        for act in self.list_action_descriptions() {
            self.set_accels_for_action(&act, &[]);
        }
    }

    pub fn add_accels(&self) {
        self.set_accels_for_action("window.close", &["<Alt>F4"]);
        self.set_accels_for_action("app.new-window", &["<Primary>N"]);
        self.set_accels_for_action("app.preferences", &["<Primary>comma"]);
        self.set_accels_for_action("app.reload-connections", &["<Primary>R"]);
        self.set_accels_for_action("win.show-help-overlay", &["<Primary>question"]);
        self.set_accels_for_action("win.fullscreen", &["F11"]);
        self.set_accels_for_action("win.show-sidebar", &["<Primary>E"]);
        self.set_accels_for_action("view.close", &["<Shift><Primary>W"]);
        self.set_accels_for_action("view.term-copy", &["<Shift><Primary>C"]);
        self.set_accels_for_action("view.term-paste", &["<Shift><Primary>V"]);
        self.set_accels_for_action("view.term-select-all", &["<Shift><Primary>A"]);
        self.set_accels_for_action("view.term-zoom-in", &["<Primary>plus"]);
        self.set_accels_for_action("view.term-zoom-out", &["<Primary>minus"]);
        self.set_accels_for_action("view.term-zoom-reset", &["<Primary>0"]);
    }

    pub fn open_new_window(&self) -> FieldMonitorWindow {
        let win = FieldMonitorWindow::new(self);
        win.present();
        win
    }

    pub fn open_preferences(&self) {
        let window = self.active_window();
        let dialog = FieldMonitorPreferencesDialog::new(self);
        dialog.present(window.as_ref());
    }

    async fn connections_dir(&self) -> PathBuf {
        let dir = user_config_dir().join("field-monitor").join("connections");
        create_dir_all(&dir).await.ok();
        dir
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        let reload_connections_action = gio::ActionEntry::builder("reload-connections")
            .activate(move |app: &Self, _, _| {
                glib::spawn_future_local(glib::clone!(
                    #[weak]
                    app,
                    async move {
                        app.reload_connections().await;
                    }
                ));
            })
            .build();
        let add_connection_action = gio::ActionEntry::builder("add-connection")
            .activate(move |app: &Self, _, _| app.add_connection_via_dialog())
            .build();
        let edit_connection_action = gio::ActionEntry::builder("edit-connection")
            .parameter_type(Some(&String::static_variant_type()))
            .activate(move |app: &Self, _, connection_id| {
                app.edit_connection_via_dialog(connection_id)
            })
            .build();
        let remove_connection_action = gio::ActionEntry::builder("remove-connection")
            .parameter_type(Some(&String::static_variant_type()))
            .activate(move |app: &Self, _, connection_id| {
                app.remove_connection_via_dialog(connection_id)
            })
            .build();
        let auth_connection_action = gio::ActionEntry::builder("auth-connection")
            .parameter_type(Some(&String::static_variant_type()))
            .activate(move |app: &Self, _, connection_id| {
                app.auth_connection_via_dialog(connection_id)
            })
            .build();
        let connect_to_server_action = gio::ActionEntry::builder("connect-to-server")
            .parameter_type(Some(&*<(String, String)>::static_variant_type()))
            .activate(move |app: &Self, _, connection_id| {
                let Some((path, adapter_id)) =
                    connection_id.and_then(<(String, String)>::from_variant)
                else {
                    warn!("Invalid parameters passed to app.connect-to-server. Ignoring.");
                    return;
                };
                if app.busy() {
                    warn!("Server action still pending. Action ignored.");
                    return;
                }
                let pending_guard = app.be_busy();
                glib::spawn_future_local(glib::clone!(
                    #[weak]
                    app,
                    async move {
                        app.connect_to_server(&path, &adapter_id).await;
                        drop(pending_guard);
                    }
                ));
            })
            .build();
        let perform_connection_action_action =
            gio::ActionEntry::builder("perform-connection-action")
                .parameter_type(Some(&*<(bool, String, String)>::static_variant_type()))
                .activate(move |app: &Self, _, connection_id| {
                    let Some((is_server, entity_path, action_id)) =
                        connection_id.and_then(<(bool, String, String)>::from_variant)
                    else {
                        warn!(
                            "Invalid parameters passed to app.perform-connection-action. Ignoring."
                        );
                        return;
                    };
                    if app.busy() {
                        warn!("Connection action still pending. Action ignored.");
                        return;
                    }
                    let pending_guard = app.be_busy();
                    glib::spawn_future_local(glib::clone!(
                        #[weak]
                        app,
                        async move {
                            app.perform_connection_action(is_server, &entity_path, &action_id)
                                .await;
                            drop(pending_guard);
                        }
                    ));
                })
                .build();
        let new_window_action = gio::ActionEntry::builder("new-window")
            .activate(move |app: &Self, _, _| {
                app.open_new_window();
            })
            .build();
        let preferences_action = gio::ActionEntry::builder("preferences")
            .activate(move |app: &Self, _, _| {
                app.open_preferences();
            })
            .build();

        self.add_action_entries([
            quit_action,
            about_action,
            reload_connections_action,
            add_connection_action,
            edit_connection_action,
            remove_connection_action,
            auth_connection_action,
            connect_to_server_action,
            perform_connection_action_action,
            new_window_action,
            preferences_action,
        ]);
    }

    /// Mark app as being busy with an action or task. This inhibits some actions and may disable
    /// some UI elements and/or show a loading indicator. Dropping the returned guard
    /// may remove the busy status (if no other source makes the app busy).
    pub fn be_busy(&self) -> BusyGuard {
        let brw = self.imp().busy_stack.borrow();
        brw.as_ref().unwrap().busy()
    }

    fn show_about(&self) {
        let window = self.active_window();

        let about = adw::AboutDialog::builder()
            .application_name(gettext("Field Monitor"))
            .application_icon(APP_ID)
            .license_type(gtk::License::Gpl30)
            .developer_name("Marco Köpcke")
            .version(VERSION)
            .developers(vec!["Marco Köpcke <hello@capypara.de>"])
            .artists(vec!["Jakub Steiner"])
            .copyright("© 2024 Marco Köpcke")
            .website("https://github.com/theCapypara/field-monitor")
            .issue_url("https://github.com/theCapypara/field-monitor/issues")
            .support_url("https://matrix.to/#/#fieldmonitor:matrix.org")
            .translator_credits(gettext(
                // Translators: Add yourself here. Format: YOUR NAME <YOUR@EMAIL.TLD>
                "translator-credits",
            ))
            .build();

        about.present(window.as_ref());
    }

    fn show_parentless_ok_dialog(&self, msg: &str) {
        let alert = adw::AlertDialog::builder().body(msg).build();
        alert.add_response("ok", &gettext("OK"));
        alert.present(None::<&gtk::Widget>);
    }

    fn show_toast_or_parentless_dialog_on_signal(
        &self,
        obj: &impl IsA<gtk::Widget>,
        signal: &str,
        window: Option<gtk::Window>,
        msg: String,
    ) {
        match window {
            Some(window) => obj.connect_closure(
                signal,
                false,
                glib::closure_local!(
                    #[watch]
                    window,
                    move |_: &gtk::Widget| {
                        if let Some(window) = window.downcast_ref::<FieldMonitorWindow>() {
                            window.toast(&msg);
                        }
                    }
                ),
            ),
            None => obj.connect_closure(
                signal,
                false,
                glib::closure_local!(
                    #[watch(rename_to=slf)]
                    self,
                    move |_: &gtk::Widget| {
                        slf.show_parentless_ok_dialog(&msg);
                    }
                ),
            ),
        };
    }

    fn add_connection_via_dialog(&self) {
        let window = self.active_window();
        let dialog = FieldMonitorAddConnectionDialog::new(self);
        let msg = gettext("Connection successfully added.");

        self.show_toast_or_parentless_dialog_on_signal(
            &dialog,
            "finished-adding",
            window.clone(),
            msg,
        );

        dialog.present(window.as_ref());
    }

    fn edit_connection_via_dialog(&self, target: Option<&glib::Variant>) {
        debug!("app.edit-connection: {:?}", target);
        let imp = self.imp();

        let Some(connection_id) = target.and_then(glib::Variant::str) else {
            warn!("Invalid connection ID target passed to app.edit-connection. Ignoring.");
            return;
        };
        let Some(connection) = imp
            .connections
            .borrow()
            .as_ref()
            .and_then(|m| m.get(connection_id).cloned())
        else {
            warn!("Connection passed to app.edit-connection not found. Ignoring.");
            return;
        };

        let window = self.active_window();
        let dialog = FieldMonitorUpdateConnectionDialog::new(self, connection);
        let msg = gettext("Connection successfully updated.");

        self.show_toast_or_parentless_dialog_on_signal(
            &dialog,
            "finished-updating",
            window.clone(),
            msg,
        );

        dialog.present(window.as_ref());
    }

    fn remove_connection_via_dialog(&self, target: Option<&glib::Variant>) {
        debug!("app.remove-connection: {:?}", target);
        let imp = self.imp();

        let Some(connection_id) = target.and_then(glib::Variant::str).map(ToString::to_string)
        else {
            warn!("Invalid connection ID target passed to app.remove-connection. Ignoring.");
            return;
        };
        let Some(connection) = imp
            .connections
            .borrow()
            .as_ref()
            .and_then(|m| m.get(&*connection_id).cloned())
        else {
            warn!("Connection passed to app.remove-connection not found. Ignoring.");
            return;
        };

        let title = connection.title();

        let window = self.active_window();
        let dialog = adw::AlertDialog::builder()
            .heading(gettext_f(
                // Translators: Do NOT translate the content between '{' and '}', this is a
                // variable name.
                "Remove {title}?",
                &[("title", &title)],
            ))
            .build();
        dialog.add_response("No", &gettext("No"));
        dialog.add_response("Yes", &gettext("Yes"));
        dialog.set_response_appearance("Yes", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("No"));
        dialog.set_close_response("No");

        let msg = gettext("Connection successfully removed.");

        if let Some(window) = window.clone() {
            dialog.connect_closure(
                "response",
                false,
                glib::closure_local!(
                    #[watch]
                    window,
                    #[weak(rename_to = slf)]
                    self,
                    move |_: &adw::AlertDialog, response: &str| {
                        if response == "Yes" {
                            slf.remove_connection(&connection_id, true);
                            if let Some(window) = window.downcast_ref::<FieldMonitorWindow>() {
                                window.toast(&msg);
                            }
                        }
                    }
                ),
            );
        } else {
            dialog.connect_closure(
                "response",
                false,
                glib::closure_local!(
                    #[watch(rename_to = slf)]
                    self,
                    move |_: &adw::AlertDialog, response: &str| {
                        if response == "Yes" {
                            slf.remove_connection(&connection_id, true);
                            slf.show_parentless_ok_dialog(&msg);
                        }
                    }
                ),
            );
        }
        dialog.present(window.as_ref());
    }

    fn auth_connection_via_dialog(&self, target: Option<&glib::Variant>) {
        debug!("app.auth-connection: {:?}", target);
        let imp = self.imp();

        let Some(connection_id) = target.and_then(glib::Variant::str).map(ToString::to_string)
        else {
            warn!("Invalid connection ID target passed to app.auth-connection. Ignoring.");
            return;
        };
        let Some(connection) = imp
            .connections
            .borrow()
            .as_ref()
            .and_then(|m| m.get(&*connection_id).cloned())
        else {
            warn!("Connection passed to app.auth-connection not found. Ignoring.");
            return;
        };

        let window = self.active_window();
        let dialog = FieldMonitorAuthenticateConnectionDialog::new(self, connection, &[]);
        let msg = gettext("Authentication successfully updated");

        self.show_toast_or_parentless_dialog_on_signal(
            &dialog,
            "auth-finished",
            window.clone(),
            msg,
        );

        dialog.present(window.as_ref());
    }

    // TODO: This should be OK here, but we should confirm.
    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn connect_to_server(&self, path: &str, adapter_id: &str) -> Option<()> {
        let imp = self.imp();
        let mut window = self
            .active_window()
            .map(Cast::downcast)
            .map(Result::unwrap)
            .unwrap_or_else(|| self.open_new_window());

        // TODO: We could also check all windows for the connection, not just the open one, but
        //       this is probably better? That way the user CAN still connect twice to a server
        //       if they really want to.
        // If already open in current window: Focus and select instead.
        if window.focus_connection_view(path, adapter_id) {
            return Some(());
        }

        let loader = ConnectionLoader::load_server(
            imp.connections.borrow().deref().as_ref(),
            Some(window.upcast_ref()),
            path,
            Some(self.clone()),
        )
        .await?;

        // If this setting is enabled open a new window to place the view into.
        if self.settings().as_ref().unwrap().open_in_new_window() {
            window = self.open_new_window();
        }

        window.open_connection_view(RemoteServerInfo::new(
            path.into(),
            adapter_id.into(),
            Cow::Borrowed(&loader.server_title().await),
            Cow::Borrowed(&loader.connection_title().await),
            loader,
        ));

        Some(())
    }

    // TODO: This should be OK here, but we should confirm.
    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn perform_connection_action(
        &self,
        is_server: bool,
        path: &str,
        action_id: &str,
    ) -> Option<()> {
        debug!("perform-connection-action: {is_server}, {path}, {action_id}");
        let imp = self.imp();
        let window = self.active_window();

        let loader = ConnectionLoader::load(
            is_server,
            imp.connections.borrow().deref().as_ref(),
            window.as_ref(),
            path,
            Some(self.clone()),
        )
        .await?;

        let action = loader.action(action_id)?;
        debug!("executing action...");
        action
            .execute(
                window.clone().as_ref(),
                window
                    .and_downcast::<FieldMonitorWindow>()
                    .map(|w| w.toast_overlay().clone())
                    .as_ref(),
            )
            .await;
        debug!("action executed");
        Some(())
    }

    pub(crate) fn connection_providers(
        &self,
    ) -> impl IntoIterator<Item = Rc<Box<dyn ConnectionProvider>>> {
        self.imp()
            .providers
            .borrow()
            .values()
            .cloned()
            .collect::<Vec<_>>()
    }

    pub fn reserve_new_connection(
        &self,
        provider: &(impl ConnectionProvider + ?Sized),
    ) -> ConnectionConfiguration {
        ConnectionConfiguration::new(
            Uuid::now_v7().to_string(),
            provider.tag().to_string(),
            self.imp().secret_manager.borrow().as_ref().unwrap().clone(),
        )
    }

    /// Returns an iterator of all currently known connections.
    pub fn connections(&self) -> impl IntoIterator<Item = ConnectionInstance> {
        let brw = self.imp().connections.borrow();
        match brw.as_ref() {
            None => HashMap::new().into_values(),
            Some(brw) => brw.clone().into_values(),
        }
    }

    /// Checks if a connection is known.
    pub fn has_connection(&self, id: &str) -> bool {
        let brw = self.imp().connections.borrow();
        brw.as_ref()
            .map(|cs| cs.contains_key(id))
            .unwrap_or_default()
    }

    /// Returns a connection by ID, if it exists.
    pub fn connection(&self, id: &str) -> Option<ConnectionInstance> {
        let brw = self.imp().connections.borrow();
        brw.as_ref().and_then(|cs| cs.get(id).cloned())
    }

    /// Updates or adds a new configuration, does not block, will run asynchronously in background.
    /// When done, the signal connection-updated is emitted.
    /// If the connection provider was not found, the connection is ignored.
    pub fn update_connection_eventually(&self, connection: DualScopedConnectionConfiguration) {
        let slf = self;
        glib::spawn_future_local(glib::clone!(
            #[weak]
            slf,
            async move { slf.update_connection(connection).await }
        ));
    }

    /// Updates or adds a new configuration.
    /// When done, the signal connection-updated is emitted.
    /// If the connection provider was not found, the connection is ignored.
    #[allow(clippy::await_holding_refcell_ref)]
    /// TODO with Rust 1.81 replace with:
    //#[expect(
    //    clippy::await_holding_refcell_ref,
    //    reason = "OK because we explicitly drop. See known problems of lint."
    //)]
    pub async fn update_connection(&self, connection: DualScopedConnectionConfiguration) {
        let _busy = self.be_busy();
        debug!("adding connection {}", connection.session().id());

        let imp = self.imp();
        let brw = imp.connections.borrow();
        let connections = brw.as_ref().unwrap();
        let connection_id = connection.session().id().to_string();
        let entry = connections.get(&connection_id);

        let Some(provider) = imp.get_provider(connection.session().tag()) else {
            warn!(
                "unknown connection provider tag {}",
                connection.session().tag()
            );
            return;
        };

        let instance = match entry {
            Some(entry) => {
                let instance = entry.clone();
                drop(brw);
                instance.set_configuration(connection).await;
                instance
            }
            None => {
                drop(brw);
                let instance = ConnectionInstance::new(connection, provider).await;
                let mut brw_mut = imp.connections.borrow_mut();
                let connections = brw_mut.as_mut().unwrap();
                connections.insert(connection_id.clone(), instance.clone());
                instance
            }
        };
        assert_eq!(&connection_id, instance.connection_id().as_str());

        self.emit_by_name::<()>("connection-updated", &[&instance]);
    }

    async fn update_connection_by_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let _busy = self.be_busy();
        let connection_id = path
            .file_stem()
            .ok_or_else(|| anyhow!("Connection file had no filename."))?
            .to_string_lossy();
        let content = read_to_string(path).await?;
        let saved_config: SavedConnectionConfiguration = serde_yaml::from_str(&content)?;
        let secret_manager = self.imp().secret_manager.borrow().as_ref().unwrap().clone();
        self.update_connection(DualScopedConnectionConfiguration::new_unified(
            ConnectionConfiguration::new_existing(
                connection_id.into_owned(),
                saved_config.tag,
                saved_config.config,
                secret_manager,
            ),
        ))
        .await;
        Ok(())
    }

    /// Removes a connection (or does nothing if the connection was not added before).
    pub fn remove_connection(&self, connection_id: &str, from_disk: bool) {
        let _busy = self.be_busy();
        debug!("removing connection {connection_id}");
        let mut brw = self.imp().connections.borrow_mut();
        if let Some(map) = brw.as_mut() {
            if map.remove(connection_id).is_some() {
                if from_disk {
                    let connection_id = connection_id.to_string();
                    glib::spawn_future_local(glib::clone!(
                        #[strong(rename_to=slf)]
                        self,
                        async move {
                            // TODO: We should probably give some visual feedback if deleting fails,
                            // even if its going to be rare. This entire function should probably
                            // be async then and propagate the error.
                            info!("Removing connection {connection_id} from disk...");
                            let mut filename = slf.connections_dir().await;
                            filename.push(format!("{}.yaml", connection_id));
                            remove_file(filename).await.ok();
                        }
                    ));
                }
                self.emit_by_name::<()>("connection-removed", &[&connection_id]);
            }
        }
    }

    /// Reloads all connections. I/O errors and config deserialization errors are logged but ignored.
    pub async fn reload_connections(&self) {
        let _busy = self.be_busy();
        debug!("reloading connections");
        self.imp().set_loading_connection(true);

        // Remove already loaded connections
        let connections_to_remove = {
            let connections_brw = self.imp().connections.borrow();
            if let Some(connections) = connections_brw.as_ref() {
                connections
                    .values()
                    .map(|con| con.connection_id())
                    .collect()
            } else {
                vec![]
            }
        };
        for connection_id in connections_to_remove.into_iter() {
            self.remove_connection(&connection_id, false);
        }

        match read_dir(self.connections_dir().await).await {
            Ok(dir) => {
                dir.for_each_concurrent(5, |dir_entry_res| async {
                    match dir_entry_res {
                        Ok(dir_entry) => {
                            let path = dir_entry.path();
                            let ext = path.extension().map(|s| s.to_string_lossy());
                            if ext != Some(Cow::Borrowed("yaml")) {
                                warn!(
                                    "Skipped file in connections without 'yaml' extension: {}",
                                    path.display()
                                );
                                return;
                            }
                            debug!("processing connection file {}", path.display());
                            if let Err(err) = self.update_connection_by_file(&path.into()).await {
                                error!(
                                    "Failed to read connection {}: {}",
                                    dir_entry.file_name().to_string_lossy(),
                                    err
                                );
                            }
                        }
                        Err(err) => {
                            error!("Failed to read a connection while iterating: {}", err);
                        }
                    }
                })
                .await;
            }
            Err(err) => {
                error!("Failed to read connections settings directory: {err}");
            }
        }
        debug!("reloading connections done");
        self.imp().set_loading_connection(false);
    }

    /// Reloads a single connections.
    pub async fn reload_connection(&self, id: &str) {
        let _busy = self.be_busy();
        debug!("reloading connection {id}");

        let file_path = self.connections_dir().await.join(format!("{id}.yaml"));
        if !file_path.exists() {
            warn!("refusing to reload {id}: connection file does no longer exist");
            return;
        }

        debug!("processing connection file {}", file_path.display());
        if let Err(err) = self.update_connection_by_file(&file_path).await {
            error!(
                "Failed to read connection {:?}: {}",
                file_path.file_name(),
                err
            );
        }

        debug!("reloading connection done");
    }

    /// Save a connection. May fail for I/O, serialization or secret service communication reasons.
    /// If `save_now` is `false`, the update in-app may happen delayed after the method has finished.
    /// If `true`, the new connection instance is returned, unless it could not be reloaded, in which
    /// case `None` is still returned.
    pub async fn save_connection(
        &self,
        mut connection: DualScopedConnectionConfiguration,
        save_now: bool,
    ) -> anyhow::Result<Option<ConnectionInstance>> {
        let _busy = self.be_busy();
        let mut filename = self.connections_dir().await;

        let c_persistent = connection.persistent_mut();

        filename.push(format!("{}.yaml", c_persistent.id()));
        let config = c_persistent.save().await?;

        let file_existed_before = filename.exists();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&filename)
            .await?;

        match serde_yaml::to_string(&SavedConnectionConfiguration {
            tag: c_persistent.tag().to_string(),
            config,
        }) {
            Ok(value) => match file.write_all(value.as_bytes()).await {
                Ok(()) => {
                    if save_now {
                        let connection_id = connection.session().id().to_string();
                        self.update_connection(connection).await;
                        match self.connection(&connection_id) {
                            None => {
                                warn!("connection was not updated properly after save.");
                                Ok(None)
                            }
                            Some(connection_instance) => Ok(Some(connection_instance)),
                        }
                    } else {
                        self.update_connection_eventually(connection);
                        Ok(None)
                    }
                }
                Err(err) => {
                    if !file_existed_before {
                        remove_file(filename).await.ok();
                    }
                    Err(err.into())
                }
            },
            Err(err) => {
                if !file_existed_before {
                    remove_file(filename).await.ok();
                }
                Err(err.into())
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SavedConnectionConfiguration {
    tag: String,
    config: HashMap<String, serde_yaml::Value>,
}
