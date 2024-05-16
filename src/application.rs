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

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{create_dir_all, OpenOptions, remove_file};
use std::path::PathBuf;
use std::sync::Arc;

use adw::glib::user_config_dir;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{APP_ID, VERSION};
use crate::connection::configuration::ConnectionConfiguration;
use crate::connection::types::ConnectionProvider;
use crate::FieldMonitorWindow;
use crate::secrets::SecretManager;

mod imp {
    use std::sync::OnceLock;

    use glib::subclass::Signal;

    use super::*;

    #[derive(Debug, Default)]
    pub struct FieldMonitorApplication {
        pub secret_manager: RefCell<Option<Arc<SecretManager>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorApplication {
        const NAME: &'static str = "FieldMonitorApplication";
        type Type = super::FieldMonitorApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for FieldMonitorApplication {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    // This signal is emitted when connections are added or updated
                    // after the first initial load.
                    Signal::builder("connection-updated")
                        .param_types([ConnectionConfiguration::static_type()])
                        .build(),
                ]
            })
        }
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
        }
    }

    impl ApplicationImpl for FieldMonitorApplication {
        // We connect to the activate callback to create a window when the application
        // has been launched. Additionally, this callback notifies us when the user
        // tries to launch a "second instance" of the application. When they try
        // to do that, we'll just present any existing window.
        fn activate(&self) {
            let hold = self.obj().hold();
            // Init secret service if not done already.
            if self.secret_manager.borrow().is_none() {
                let slf = self;
                glib::spawn_future_local(glib::clone!(@weak slf => async move {
                    let _hold = hold;
                    let secrets = SecretManager::new().await;
                    match secrets {
                        Ok(secrets) => {
                            slf.secret_manager.replace(Some(Arc::new(secrets)));
                            slf.present_window();
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
                self.present_window();
            }
        }
    }

    impl FieldMonitorApplication {
        fn present_window(&self) {
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
    }

    impl GtkApplicationImpl for FieldMonitorApplication {}
    impl AdwApplicationImpl for FieldMonitorApplication {}
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

    fn connections_dir(&self) -> PathBuf {
        let dir = user_config_dir().join("field-monitor").join("connections");
        create_dir_all(&dir).ok();
        dir
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        self.add_action_entries([quit_action, about_action]);
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

    /// Adds a new configuration. The configuration passed in will be cloned.
    pub fn add_connection(&self, connection: &ConnectionConfiguration) {
        // todo
        // TODO: Connection updated signal
        // TODO: toast
    }

    /// Update configuration. The configuration passed in will be cloned.
    pub fn update_connection(&self, connection: &ConnectionConfiguration) {
        // todo
        // TODO: Connection updated signal
        // TODO: toast
    }

    pub async fn save_connection(
        &self,
        connection: &mut ConnectionConfiguration,
    ) -> anyhow::Result<()> {
        let mut filename = self.connections_dir();
        filename.push(format!("{}.yaml", connection.id()));
        let config = connection.save().await?;

        let file_existed_before = filename.exists();
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&filename)?;

        match serde_yaml::to_writer(
            file,
            &SavedConnectionConfiguration {
                tag: connection.tag(),
                config,
            },
        ) {
            Ok(_) => {
                if file_existed_before {
                    self.add_connection(connection);
                } else {
                    self.update_connection(connection);
                }
                Ok(())
            }
            Err(err) => {
                if !file_existed_before {
                    remove_file(filename).ok();
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
