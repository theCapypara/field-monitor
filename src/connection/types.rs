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
use std::rc::Rc;

use futures::future::LocalBoxFuture;
use futures::lock::Mutex;

use crate::adapter::types::Adapter;
use crate::application::FieldMonitorApplication;
use crate::connection::configuration::ConnectionConfiguration;

/// Metadata about a connection.
#[derive(Debug, Clone)]
pub struct ConnectionMetadata {}

/// Metadata about a server.
#[derive(Debug, Clone)]
pub struct ServerMetadata {}

/// Constructor for ConnectionProvider and static members for ConnectionProviders.
/// This is separate to allow trait objects / dynamic dispatch.
pub trait ConnectionProviderConstructor: Send + Sync {
    /// Creates the provider.
    #[allow(clippy::new_ret_no_self, clippy::wrong_self_convention)]
    fn new(&self, app: &FieldMonitorApplication) -> Box<dyn ConnectionProvider>;
}

/// A provider for creating new connections. Each provider can create new connections
/// of a defined type.
pub trait ConnectionProvider {
    /// Tag for configuration connection. Will be serialized with configuration
    /// and used to match connection providers.
    fn tag(&self) -> &'static str;

    /// Title describing the provider.
    fn title(&self) -> Cow<str>;

    /// Plural title of the provider. This may describe a group of multiple connections created
    /// by the provider.
    fn title_plural(&self) -> Cow<str>;

    /// Title to display when showing the dialog to add a new connection of this type.
    fn add_title(&self) -> Cow<str>;

    /// Returns a description to be shown in the "add new connection" dialog.
    fn description(&self) -> Cow<str>;

    /// Creates a preference page (or other applicable widget) for configuring a new connection.
    ///
    /// If this is for modifying an existing configuration, the current configuration is set.
    fn preferences(&self, configuration: Option<Rc<Mutex<ConnectionConfiguration>>>)
        -> gtk::Widget;

    /// Update a connection configuration from a configured preference page.
    /// It may be asserted that the  passed `preferences` are a widget returned from `preferences`.
    ///
    /// If an error is returned, the preferences will still be shown to the user. This means
    /// the implementation may modify the preferences to hint at input errors. It should not show
    /// the error message of the returned result, this is presented by the caller.
    fn update_connection(
        &self,
        preferences: gtk::Widget,
        configuration: Rc<Mutex<ConnectionConfiguration>>,
    ) -> LocalBoxFuture<anyhow::Result<()>>;

    /// Creates a preference group (or another applicable widgets, such as a box to group multiple)
    /// for configuring credentials.
    ///
    /// This is shown to the user when a connection with the stored credentials
    /// could not be made, or when no credentials were stored.
    ///
    /// The passed parameter contains the current configuration before.
    fn configure_credentials(
        &self,
        configuration: Rc<Mutex<ConnectionConfiguration>>,
    ) -> gtk::Widget;

    /// Update the credentials of a connection.
    /// It may be asserted that the  passed `preferences` are a widget returned from
    /// `configure_credentials`.
    ///
    /// If an error is returned, the preferences will still be shown to the user. This means
    /// the implementation may modify the preferences to hint at input errors. It should not show
    /// the error message of the returned result, this is presented by the caller.
    fn store_credentials(
        &self,
        preferences: gtk::Widget,
        configuration: Rc<Mutex<ConnectionConfiguration>>,
    ) -> LocalBoxFuture<anyhow::Result<()>>;

    /// Try to load a connection configuration into a connection.
    /// The tag inside the configuration must match [`Self::TAG`], otherwise the method
    /// may error or incorrectly try to load.
    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> anyhow::Result<Box<dyn Connection>>;
}

/// A connection. Represents one or more servers which are logically
/// grouped together.
///
/// It manages zero, one or multiple servers.
pub trait Connection {
    /// Metadata about the connection.
    fn metadata(&self) -> &ConnectionMetadata;

    /// Returns the servers managed by this connection.
    fn servers(&self) -> anyhow::Result<&[&dyn ServerConnection]>;
}

/// A single instance of a server to connect to.
/// It may contain sub-servers.
pub trait ServerConnection {
    /// Metadata about the server.
    fn metadata(&self) -> &ServerMetadata;

    /// List of adapters that can be used to connect to the server.
    fn adapters(&self) -> &[&dyn Adapter];

    /// Returns the sub-servers grouped under this server.
    fn servers(&self) -> anyhow::Result<&[&dyn ServerConnection]> {
        Ok(&[])
    }
}
