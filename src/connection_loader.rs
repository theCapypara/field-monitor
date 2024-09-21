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
use std::collections::HashMap;
use std::ops::Deref;

use adw::prelude::{AdwDialogExt, AlertDialogExt};
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use log::{debug, error, warn};

use libfieldmonitor::connection::*;

enum Entity {
    Connection(ConnectionInstance),
    Server(Box<dyn ServerConnection>),
}

/// Loads connections and gets resources from them. Interactively shows error messages to the user
/// and short-circuits any failures or not found resources by returning None.
pub(super) struct ConnectionLoader {
    entity: Entity,
    window: Option<gtk::Window>,
}

impl ConnectionLoader {
    #[allow(unused)]
    pub async fn load_connection(
        connections: impl Deref<Target = Option<HashMap<String, ConnectionInstance>>>,
        active_window: Option<&gtk::Window>,
        path: &str,
    ) -> Option<Self> {
        Self::load(false, connections, active_window, path).await
    }

    pub async fn load_server(
        connections: impl Deref<Target = Option<HashMap<String, ConnectionInstance>>>,
        active_window: Option<&gtk::Window>,
        path: &str,
    ) -> Option<Self> {
        Self::load(true, connections, active_window, path).await
    }

    pub async fn load(
        is_server: bool,
        connections: impl Deref<Target = Option<HashMap<String, ConnectionInstance>>>,
        active_window: Option<&gtk::Window>,
        path: &str,
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

        if !is_server {
            Some(Self {
                entity: Entity::Connection(connection),
                window: active_window.cloned(),
            })
        } else {
            let mut servers =
                Some(Self::do_load_server(|| connection.servers(), active_window.cloned()).await?);

            let mut server: Option<Box<dyn ServerConnection>> = None;

            for path_part in path_parts {
                // get subservers
                // this will only be relevant on the second+ loop, on the first we still have the list
                // from servers of the connection.
                if let Some(server) = server {
                    servers =
                        Self::do_load_server(|| server.servers(), active_window.cloned()).await;
                }

                // check if subserver list is even set.
                let Some(ref mut servers_rf) = servers else {
                    warn!("servers were empty");
                    Self::do_show_error(&gettext("Server not found."), None, active_window);
                    return None;
                };

                // get subserver from server list.
                server = servers_rf.swap_remove(path_part);
            }

            match server {
                None => {
                    warn!("server not found");
                    Self::do_show_error(&gettext("Server not found."), None, active_window);
                    None
                }
                Some(server) => Some(Self {
                    entity: Entity::Server(server),
                    window: active_window.cloned(),
                }),
            }
        }
    }

    fn do_load_server<'a, 'b, F>(
        mk_servers_fut: F,
        active_window: Option<gtk::Window>,
    ) -> LocalBoxFuture<'a, Option<ServerMap>>
    where
        F: 'b + Fn() -> LocalBoxFuture<'a, ConnectionResult<ServerMap>>,
        'b: 'a,
    {
        Box::pin(async move {
            match mk_servers_fut().await {
                Ok(servers) => Some(servers),
                Err(ConnectionError::AuthFailed(_, _)) => {
                    warn!("auth failed, asking to re-auth");
                    Self::handle_auth_needed().await;
                    match mk_servers_fut().await {
                        Ok(servers) => Some(servers),
                        Err(ConnectionError::AuthFailed(msg, details))
                        | Err(ConnectionError::General(msg, details)) => {
                            warn!("failed to load servers after re-auth: {msg:?} - {details}");
                            Self::do_show_error(
                                &gettext("Failed to load or connect to server."),
                                msg.as_deref(),
                                active_window.as_ref(),
                            );
                            None
                        }
                    }
                }
                Err(ConnectionError::General(msg, details)) => {
                    warn!("failed to load servers: {msg:?} - {details}");
                    Self::do_show_error(
                        &gettext("Failed to load or connect to server."),
                        msg.as_deref(),
                        active_window.as_ref(),
                    );
                    None
                }
            }
        })
    }

    pub fn action(&self, action_id: &str) -> Option<ServerAction> {
        match &self.entity {
            Entity::Connection(e) => e.actions().swap_remove(action_id),
            Entity::Server(e) => e.actions().swap_remove(action_id),
        }
    }

    async fn handle_auth_needed() {
        error!("TODO auth in ConnectionLoader");
        todo!();
    }

    #[allow(unused)]
    fn show_error(&self, title: &str, msg: Option<&str>) {
        Self::do_show_error(title, msg, self.window.as_ref())
    }

    fn do_show_error(title: &str, msg: Option<&str>, window: Option<&gtk::Window>) {
        let alert = adw::AlertDialog::builder().title(title).build();
        if let Some(msg) = msg {
            alert.set_body(msg);
        }
        alert.add_response("ok", &gettext("OK"));
        alert.present(window);
    }
}
