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
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

use adw::prelude::{AdwDialogExt, AlertDialogExt};
use futures::channel::oneshot;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use gtk::prelude::*;
use log::{debug, warn};

use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::connection::*;

use crate::application::FieldMonitorApplication;
use crate::widget::authenticate_connection_dialog::FieldMonitorAuthenticateConnectionDialog;

enum Entity {
    Connection(ConnectionInstance),
    Server(Box<dyn ServerConnection>),
}

/// Loads connections and gets resources from them. Interactively shows error messages to the user
/// and short-circuits any failures or not found resources by returning None.
pub struct ConnectionLoader {
    entity: Entity,
    connection: ConnectionInstance,
    window: Option<gtk::Window>,
    app: Option<FieldMonitorApplication>,
    server_path: Vec<String>,
}

impl ConnectionLoader {
    #[allow(unused)]
    pub async fn load_connection(
        connections: impl Deref<Target = Option<HashMap<String, ConnectionInstance>>>,
        active_window: Option<&gtk::Window>,
        path: &str,
        app: Option<FieldMonitorApplication>,
    ) -> Option<Self> {
        Self::load(false, connections, active_window, path, app).await
    }

    pub async fn load_server(
        connections: impl Deref<Target = Option<HashMap<String, ConnectionInstance>>>,
        active_window: Option<&gtk::Window>,
        path: &str,
        app: Option<FieldMonitorApplication>,
    ) -> Option<Self> {
        Self::load(true, connections, active_window, path, app).await
    }

    pub async fn load(
        is_server: bool,
        connections: impl Deref<Target = Option<HashMap<String, ConnectionInstance>>>,
        active_window: Option<&gtk::Window>,
        path: &str,
        app: Option<FieldMonitorApplication>,
    ) -> Option<Self> {
        let connections = connections.deref();
        debug!("Loading connection or server for an event: is server={is_server}; path={path}");
        let mut path_parts = path.split("/");

        let Some(connection_id) = path_parts.next() else {
            warn!("path had no connection ID");
            Self::do_show_error(&gettext("Connection not found."), None, active_window);
            return None;
        };

        let Some(connection) = connections
            .as_ref()
            .and_then(|cs| cs.get(connection_id))
            .cloned()
        else {
            warn!("connection not found");
            Self::do_show_error(&gettext("Connection not found."), None, active_window);
            return None;
        };

        Self::do_load_connection(
            is_server,
            connection,
            active_window,
            path_parts.map(ToOwned::to_owned).collect(),
            app,
            true,
        )
        .await
    }

    fn do_load_connection(
        is_server: bool,
        connection: ConnectionInstance,
        active_window: Option<&gtk::Window>,
        path_parts: Vec<String>,
        app: Option<FieldMonitorApplication>,
        try_reauth: bool,
    ) -> LocalBoxFuture<Option<Self>> {
        Box::pin(async move {
            let path_parts_orig = path_parts;
            let path_parts = path_parts_orig.iter();

            if !is_server {
                Some(Self {
                    entity: Entity::Connection(connection.clone()),
                    connection,
                    window: active_window.cloned(),
                    app,
                    server_path: vec![],
                })
            } else {
                // Get the servers from the connection. On auth failure, restart.
                let mut servers = match Self::do_load_server(
                    connection.servers(),
                    connection.clone(),
                    &path_parts_orig,
                    app.clone(),
                    active_window.cloned(),
                    try_reauth,
                )
                .await
                {
                    Ok(servers_new) => Some(servers_new),
                    Err(None) => return None,
                    Err(Some(connection)) => {
                        // restart.
                        return Self::do_load_connection(
                            is_server,
                            connection,
                            active_window,
                            path_parts_orig,
                            app,
                            false,
                        )
                        .await;
                    }
                };

                let mut server: Option<Box<dyn ServerConnection>> = None;

                for path_part in path_parts {
                    // get subservers
                    // this will only be relevant on the second+ loop, on the first we still have the list
                    // from servers of the connection.
                    if let Some(server) = server {
                        match Self::do_load_server(
                            server.servers(),
                            connection.clone(),
                            &path_parts_orig,
                            app.clone(),
                            active_window.cloned(),
                            try_reauth,
                        )
                        .await
                        {
                            Ok(servers_new) => servers = Some(servers_new),
                            Err(None) => servers = None,
                            Err(Some(connection)) => {
                                // restart.
                                return Self::do_load_connection(
                                    is_server,
                                    connection,
                                    active_window,
                                    path_parts_orig,
                                    app,
                                    false,
                                )
                                .await;
                            }
                        }
                    }

                    // check if subserver list is even set.
                    let Some(ref mut servers_rf) = servers else {
                        warn!("servers were empty");
                        Self::do_show_error(&gettext("Server not found."), None, active_window);
                        return None;
                    };

                    // get subserver from server list.
                    server = servers_rf.swap_remove(&Cow::Borrowed(path_part.as_str()));
                }

                match server {
                    None => {
                        warn!("server not found");
                        Self::do_show_error(&gettext("Server not found."), None, active_window);
                        None
                    }
                    Some(server) => Some(Self {
                        entity: Entity::Server(server),
                        connection,
                        window: active_window.cloned(),
                        app,
                        server_path: path_parts_orig,
                    }),
                }
            }
        })
    }

    /// Tries to load a server.
    /// Success: Ok(ServerMap)
    /// Auth Error: Err(Some(ConnectionInstance)) ( the instance to may try again with )
    /// General error: Err(None) ( give up; an error is already shown. )
    async fn do_load_server(
        servers_fut: LocalBoxFuture<'_, ConnectionResult<ServerMap>>,
        connection: ConnectionInstance,
        server_path: &[String],
        app: Option<FieldMonitorApplication>,
        active_window: Option<gtk::Window>,
        try_reauth: bool,
    ) -> Result<ServerMap, Option<ConnectionInstance>> {
        match servers_fut.await {
            Ok(servers) => Ok(servers),
            Err(ConnectionError::AuthFailed(_, _)) if try_reauth => {
                warn!("auth failed, asking to re-auth");
                let connection =
                    Self::handle_auth_needed(connection, server_path, app, active_window)
                        .await
                        .unwrap();
                debug!("reauth finished");
                Err(Some(connection))
            }
            Err(ConnectionError::General(msg, details))
            | Err(ConnectionError::AuthFailed(msg, details)) => {
                warn!("failed to load servers: {msg:?} - {details}");
                Self::do_show_error(
                    &gettext("Failed to load or connect to server"),
                    msg.as_deref(),
                    active_window.as_ref(),
                );
                Err(None)
            }
        }
    }

    /// Gets the name of the server. Panics if this is not for a server.
    pub fn server_title(&self) -> String {
        match &self.entity {
            Entity::Server(server) => server.metadata().title,
            _ => panic!("ConnectionLoader is not for server - but server named asked"),
        }
    }

    /// Gets the name of the connection.
    pub fn connection_title(&self) -> String {
        self.connection.metadata().title
    }

    pub fn action(&self, action_id: &str) -> Option<ServerAction> {
        match &self.entity {
            Entity::Connection(e) => e.action(action_id),
            Entity::Server(e) => e.action(action_id),
        }
    }

    pub async fn create_adapter(
        &mut self,
        tag: &str,
        try_reauth: bool,
    ) -> Option<Box<dyn Adapter>> {
        debug!("creating adapter");
        match self.create_adapter_internal(tag, try_reauth).await {
            Ok(servers_new) => Some(servers_new),
            Err(None) => None,
            Err(Some(connection)) => {
                // Connection failed and we (potentially) re-authed.
                // Try recreating and then try again.
                debug!("recreating self and retrying");
                *self = Self::do_load_connection(
                    true,
                    connection,
                    self.window.as_ref(),
                    self.server_path.clone(),
                    self.app.clone(),
                    false,
                )
                .await?;
                self.create_adapter_internal(tag, false).await.ok()
            }
        }
    }

    pub async fn reauth(&mut self) -> Option<()> {
        debug!("forcing re-auth");

        let connection = Self::handle_auth_needed(
            self.connection.clone(),
            &self.server_path,
            self.app.clone(),
            self.window.clone(),
        )
        .await
        .unwrap();

        *self = Self::do_load_connection(
            true,
            connection,
            self.window.as_ref(),
            self.server_path.clone(),
            self.app.clone(),
            false,
        )
        .await?;

        Some(())
    }

    async fn create_adapter_internal(
        &self,
        tag: &str,
        try_reauth: bool,
    ) -> Result<Box<dyn Adapter>, Option<ConnectionInstance>> {
        match &self.entity {
            Entity::Connection(_) => panic!("an adapter can only be created for a server"),
            Entity::Server(e) => match e.create_adapter(tag).await {
                Ok(adapter) => Ok(adapter),
                Err(ConnectionError::AuthFailed(_, _)) if try_reauth => {
                    warn!("auth failed, asking to re-auth");
                    let connection = Self::handle_auth_needed(
                        self.connection.clone(),
                        self.server_path.as_slice(),
                        self.app.clone(),
                        self.window.clone(),
                    )
                    .await
                    .unwrap();
                    debug!("reauth finished");
                    Err(Some(connection))
                }
                Err(ConnectionError::General(msg, details))
                | Err(ConnectionError::AuthFailed(msg, details)) => {
                    warn!("failed to load servers: {msg:?} - {details}");
                    Self::do_show_error(
                        &gettext("Failed to load or connect to server"),
                        msg.as_deref(),
                        self.window.as_ref(),
                    );
                    Err(None)
                }
            },
        }
    }

    /// Runs the authentication dialog to update the connection, returns the connection.
    fn handle_auth_needed(
        connection: ConnectionInstance,
        server_path: &[String],
        app: Option<FieldMonitorApplication>,
        window: Option<gtk::Window>,
    ) -> oneshot::Receiver<ConnectionInstance> {
        debug!("auth in ConnectionLoader");
        let (sender, receiver) = oneshot::channel();

        if let Some(app) = app {
            let sender = Rc::new(RefCell::new(Some(sender)));
            let dialog = FieldMonitorAuthenticateConnectionDialog::new(
                &app,
                connection.clone(),
                server_path,
            );

            dialog.connect_closure(
                "closed",
                false,
                glib::closure_local!(
                    #[strong]
                    sender,
                    #[strong]
                    connection,
                    move |dialog: &FieldMonitorAuthenticateConnectionDialog| {
                        debug!("auth dialog closed");
                        let mut sender_guard = sender.borrow_mut();
                        let sender = sender_guard.take();
                        if let Some(sender) = sender {
                            sender
                                .send(dialog.saved_connection().unwrap_or(connection.clone()))
                                .ok();
                        }
                    }
                ),
            );

            dialog.present(window.as_ref());

            receiver
        } else {
            // not supported without app
            sender.send(connection).ok();
            receiver
        }
    }

    #[allow(unused)]
    fn show_error(&self, title: &str, msg: Option<&str>) {
        Self::do_show_error(title, msg, self.window.as_ref())
    }

    fn do_show_error(title: &str, msg: Option<&str>, window: Option<&gtk::Window>) {
        let alert = adw::AlertDialog::builder().heading(title).build();
        if let Some(msg) = msg {
            alert.set_body(msg);
        }
        alert.add_response("ok", &gettext("OK"));
        alert.present(window);
    }
}
