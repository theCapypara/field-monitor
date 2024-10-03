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
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use adw::glib::user_config_dir;
use adw::prelude::*;
use adw::subclass::prelude::*;
use anyhow::anyhow;
use async_std::fs::{create_dir_all, OpenOptions, read_dir, read_to_string, remove_file};
use async_std::io::WriteExt;
use futures::StreamExt;
use gettextrs::gettext;
use glib::subclass::Signal;
use gtk::{gio, glib};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use libfieldmonitor::busy::{BusyGuard, BusyStack};
use libfieldmonitor::connection::{Connection, DualScopedConnectionConfiguration};
use libfieldmonitor::connection::ConnectionConfiguration;
use libfieldmonitor::connection::ConnectionInstance;
use libfieldmonitor::connection::ConnectionProvider;
use libfieldmonitor::i18n::gettext_f;
use libfieldmonitor::ManagesSecrets;

use crate::config::{APP_ID, VERSION};
use crate::connection::CONNECTION_PROVIDERS;
use crate::connection_loader::ConnectionLoader;
use crate::secrets::SecretManager;
use crate::widget::add_connection_dialog::FieldMonitorAddConnectionDialog;
use crate::widget::authenticate_connection_dialog::FieldMonitorAuthenticateConnectionDialog;
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
        /// Whether Field Monitor is currently (re-)loading all connections.
        #[property(get)]
        pub loading_connections: Cell<bool>,
        /// Currently busy with processing an action or connection request to a server or connection.
        #[property(get)]
        pub busy: Rc<Cell<bool>>,
        #[property(get, construct_only)]
        pub settings: RefCell<Option<gio::Settings>>,
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
            // Listen to own signals for debug purposes
            self.obj().connect_closure(
                "connection-updated",
                false,
                glib::closure_local!(
                    move |_: super::FieldMonitorApplication, instance: ConnectionInstance| {
                        debug!(
                            "connection updated: tag: {:?}, id: {:?}, title: {:?}",
                            instance.provider_tag(),
                            instance.connection_id(),
                            &instance.metadata().title
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
        // We connect to the activate callback to create a window when the application
        // has been launched. Additionally, this callback notifies us when the user
        // tries to launch a "second instance" of the application. When they try
        // to do that, we'll just present any existing window.
        fn activate(&self) {
            let hold = self.obj().hold();
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
                let slf = self;
                glib::spawn_future_local(glib::clone!(
                    #[weak]
                    slf,
                    async move {
                        let _hold = hold;
                        let secrets = SecretManager::new().await;
                        match secrets {
                            Ok(secrets) => {
                                slf.secret_manager
                                    .replace(Some(Arc::new(Box::new(secrets))));
                                slf.finish_activate();
                            }
                            Err(err) => {
                                let alert = adw::MessageDialog::builder()
                                .title(gettext("Failed to initialize"))
                                .body(format!(
                                    "{}:\n{}",
                                    gettext("Field Monitor could not start, because it could not connect to your system's secret service for accessing passwords"),
                                    err
                                ))
                                .application(&*slf.obj())
                                .build();
                                alert.add_response("ok", &gettext("OK"));
                                alert.present();
                            }
                        }
                    }
                ));
            } else {
                self.finish_activate();
            }
        }
    }

    impl GtkApplicationImpl for FieldMonitorApplication {}
    impl AdwApplicationImpl for FieldMonitorApplication {}

    impl FieldMonitorApplication {
        fn finish_activate(&self) {
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
            let window = if let Some(window) = application.active_window() {
                window
            } else {
                let window = FieldMonitorWindow::new(&application);
                window.upcast()
            };

            // Ask the window manager/compositor to present the window
            window.present();
        }

        pub fn set_loading_connection(&self, value: bool) {
            self.loading_connections.replace(value);
            self.obj().notify_loading_connections();
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
        let app: FieldMonitorApplication = glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .property("settings", gio::Settings::new(APP_ID))
            .build();
        app.imp().busy_stack.borrow_mut().replace(BusyStack::new(
            app.imp().busy.clone(),
            Box::new(glib::clone!(
                #[weak]
                app,
                move || app.notify("busy")
            )),
        ));

        // Accelerators
        app.set_accels_for_action("win.fullscreen", &["F11"]);

        // Prefer dark style by default
        app.style_manager()
            .set_color_scheme(adw::ColorScheme::PreferDark);

        app
    }

    pub fn open_new_window(&self) -> FieldMonitorWindow {
        let win = FieldMonitorWindow::new(self);
        win.present();
        win
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
            .developers(vec!["Marco Köpcke"])
            .copyright("© 2024 Marco Köpcke")
            .website("https://github.com/theCapypara/FieldMonitor")
            .issue_url("https://github.com/theCapypara/FieldMonitor/issues")
            .support_url("https://matrix.to/#/#fieldmonitor:matrix.org")
            .translator_credits(gettext("translator-credits"))
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

        let title = connection.title().unwrap_or_default();

        let window = self.active_window();
        let dialog = adw::AlertDialog::builder()
            .heading(gettext_f("Remove {title}?", &[("title", &title)]))
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

    pub async fn connect_to_server(&self, path: &str, adapter_id: &str) -> Option<()> {
        let imp = self.imp();
        let window = self
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
            imp.connections.borrow(),
            Some(window.upcast_ref()),
            path,
            Some(self.clone()),
        )
        .await?;

        window.open_connection_view(
            path,
            adapter_id,
            &loader.server_title(),
            &loader.connection_title(),
            loader,
        );

        Some(())
    }

    pub async fn perform_connection_action(
        &self,
        is_server: bool,
        path: &str,
        action_id: &str,
    ) -> Option<()> {
        let imp = self.imp();
        let window = self.active_window();

        let loader = ConnectionLoader::load(
            is_server,
            imp.connections.borrow(),
            window.as_ref(),
            path,
            Some(self.clone()),
        )
        .await?;

        let action = loader.action(action_id)?;
        action
            .execute(
                window.clone().as_ref(),
                window
                    .and_then(|w| w.downcast::<FieldMonitorWindow>().ok())
                    .map(|w| w.toast_overlay().clone())
                    .as_ref(),
            )
            .await;
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
                            debug!("processing connection file {}", dir_entry.path().display());
                            if let Err(err) = self
                                .update_connection_by_file(&dir_entry.path().into())
                                .await
                            {
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
