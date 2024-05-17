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
use std::collections::hash_map::Entry;
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
use log::{debug, error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{APP_ID, VERSION};
use crate::connection::configuration::ConnectionConfiguration;
use crate::connection::CONNECTION_PROVIDERS;
use crate::connection::instance::ConnectionInstance;
use crate::connection::types::ConnectionProvider;
use crate::FieldMonitorWindow;
use crate::secrets::SecretManager;

use crate::connection::CONNECTION_PROVIDERS;
use crate::secrets::SecretManager;

mod imp {
    use log::debug;

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorApplication)]
    pub struct FieldMonitorApplication {
        pub secret_manager: RefCell<Option<Arc<SecretManager>>>,
        pub connections: RefCell<Option<HashMap<String, ConnectionInstance>>>,
        pub providers: RefCell<HashMap<String, Rc<Box<dyn ConnectionProvider>>>>,
        /// Whether Field Monitor is currently (re-)loading all connections.
        #[property(get)]
        pub loading_connections: Cell<bool>,
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
                    // This signal is emitted when connections are added or updated
                    // after the first initial load.
                    Signal::builder("connection-updated")
                        .param_types([ConnectionInstance::static_type()])
                        .build(),
                    // This signal is emitted when connections was added or updated,
                    // but it failed to initialize.
                    Signal::builder("connection-failed-updating")
                        .param_types([
                            String::static_type(), // Provider Title
                            String::static_type(), // Connection ID
                            String::static_type(), // Error
                        ])
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
                glib::closure_local!(move |instance: ConnectionInstance| {
                    debug!(
                        "connection updated: tag: {:?}, id: {:?}",
                        instance.provider_tag(),
                        instance.id()
                    );
                }),
            );
            self.obj().connect_closure(
                "connection-failed-updating",
                false,
                glib::closure_local!(move |provider_title: String, id: String, error: String| {
                    debug!(
                        "connection failed updating: provider: {}, id: {}, error: {}",
                        provider_title, id, error
                    );
                }),
            );
            self.obj().connect_closure(
                "connection-removed",
                false,
                glib::closure_local!(move |id: String| {
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
                            let provider = constructor.new(&self.obj());
                            (provider.tag().to_owned(), Rc::new(provider))
                        })
                        .collect(),
                );
            }
            // Init secret service if not done already.
            if self.secret_manager.borrow().is_none() {
                let slf = self;
                glib::spawn_future_local(glib::clone!(@weak slf => async move {
                    let _hold = hold;
                    let secrets = SecretManager::new().await;
                    match secrets {
                        Ok(secrets) => {
                            slf.secret_manager.replace(Some(Arc::new(secrets)));
                            slf.finish_activate();
                        },
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
                }));
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
                glib::spawn_future_local(glib::clone!(@weak slf => async move {
                    slf.obj().reload_connections().await;
                }));
            }

            let application = self.obj();

            // Get the current window or create one if necessary
            let window = if let Some(window) = application.active_window() {
                window
            } else {
                let window = FieldMonitorWindow::new(&*application);
                window.open_new_connection_list();
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
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build()
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
                glib::spawn_future_local(
                    glib::clone!(@weak app => async move { app.reload_connections().await; }),
                );
            })
            .build();
        self.add_action_entries([quit_action, about_action, reload_connections_action]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
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

        about.present(Some(&window));
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

    /// Updates or adds a new configuration, does not block, will run asynchronously in background.
    /// Result is communicated via signals connection-updated or connection-failed-updating.
    pub fn update_connection_eventually(&self, connection: ConnectionConfiguration) {
        let slf = self;
        glib::spawn_future_local(glib::clone!(@weak slf => async move {
            slf.update_connection(connection).await
        }));
    }

    /// Updates or adds a new configuration.
    /// Result is communicated via signals connection-updated or connection-failed-updating.
    pub async fn update_connection(&self, connection: ConnectionConfiguration) {
        debug!("adding connection {}", connection.id());
        let provider_title = self.provider_title_or_unknown(&connection);
        let connection_id = connection.id().to_string();
        match self.try_update_connection(connection).await {
            Ok(instance) => {
                self.emit_by_name::<()>("connection-updated", &[&instance]);
            }
            Err(err) => {
                self.emit_by_name::<()>(
                    "connection-failed-updating",
                    &[&provider_title, &connection_id, &err.to_string()],
                );
            }
        }
    }

    // XXX: This isn't ideal but should be OK, since this is all local.
    #[allow(clippy::await_holding_refcell_ref)]
    async fn try_update_connection(
        &self,
        connection: ConnectionConfiguration,
    ) -> anyhow::Result<ConnectionInstance> {
        let imp = self.imp();
        let mut brw = imp.connections.borrow_mut();
        let connections = brw.as_mut().unwrap();
        let entry = connections.entry(connection.id().to_string());

        let provider = imp.get_provider(connection.tag()).ok_or_else(|| {
            anyhow!(
                "{}: {}",
                gettext("Connection provider for connection type not found"),
                connection.tag()
            )
        })?;
        let instance = match entry {
            Entry::Occupied(slot) => {
                let instance = slot.into_mut();
                instance.set_configuration(connection).await?;
                instance
            }
            Entry::Vacant(slot) => {
                slot.insert(ConnectionInstance::new(connection, provider).await?)
            }
        };
        Ok(instance.clone())
    }

    async fn update_connection_by_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let connection_id = path
            .file_stem()
            .ok_or_else(|| anyhow!("Connection file had no filename."))?
            .to_string_lossy();
        let content = read_to_string(path).await?;
        let saved_config: SavedConnectionConfiguration = serde_yaml::from_str(&content)?;
        let secret_manager = self.imp().secret_manager.borrow().as_ref().unwrap().clone();
        self.update_connection(ConnectionConfiguration::new_existing(
            connection_id.into_owned(),
            saved_config.tag,
            saved_config.config,
            secret_manager,
        ))
        .await;
        Ok(())
    }

    /// Removes a connection (or does nothing if the connection was not added before).
    pub fn remove_connection(&self, connection: &ConnectionConfiguration) {
        let id = connection.id();
        let mut brw = self.imp().connections.borrow_mut();
        if let Some(map) = brw.as_mut() {
            if map.remove(id).is_some() {
                self.emit_by_name::<()>("connection-removed", &[&id]);
            }
        }
    }

    /// Reloads all connections. I/O errors and config deserialization errors are logged but ignored.
    pub async fn reload_connections(&self) {
        debug!("reloading connections");
        self.imp().set_loading_connection(true);
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
    pub async fn save_connection(
        &self,
        connection: &mut ConnectionConfiguration,
    ) -> anyhow::Result<()> {
        let mut filename = self.connections_dir().await;
        filename.push(format!("{}.yaml", connection.id()));
        let config = connection.save().await?;

        let file_existed_before = filename.exists();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&filename)
            .await?;

        match serde_yaml::to_string(&SavedConnectionConfiguration {
            tag: connection.tag().to_string(),
            config,
        }) {
            Ok(value) => match file.write_all(value.as_bytes()).await {
                Ok(()) => {
                    self.update_connection_eventually(connection.clone());
                    Ok(())
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

    fn provider_title_or_unknown(&self, connection: &ConnectionConfiguration) -> String {
        let brw = self.imp().providers.borrow();
        let provider = brw.get(connection.tag());
        match provider {
            None => gettext("Unknown"),
            Some(provider) => provider.title().into_owned(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SavedConnectionConfiguration {
    tag: String,
    config: HashMap<String, serde_yaml::Value>,
}
