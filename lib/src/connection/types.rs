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
use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

use derive_builder::Builder;
use futures::future::LocalBoxFuture;
use indexmap::IndexMap;
use thiserror::Error;

use crate::adapter::types::Adapter;
use crate::connection::configuration::ConnectionConfiguration;
use crate::connection::DualScopedConnectionConfiguration;

pub type ConnectionResult<T> = Result<T, ConnectionError>;

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("{1}")]
    /// Authentication failed and may be required
    AuthFailed(Option<String>, anyhow::Error),
    #[error("{1}")]
    /// General failure, which can not be recovered from by re-authenticating
    General(Option<String>, anyhow::Error),
}

impl ConnectionError {
    pub fn clone_outside(self: &Arc<Self>) -> Self {
        match self.as_ref() {
            Self::AuthFailed(msg, _) => {
                Self::AuthFailed(msg.clone(), anyhow::Error::from(self.clone()))
            }
            Self::General(msg, _) => Self::General(msg.clone(), anyhow::Error::from(self.clone())),
        }
    }
}

impl ConnectionError {
    pub fn auth_failed(&self) -> bool {
        match self {
            ConnectionError::AuthFailed(_, _) => true,
            ConnectionError::General(_, _) => false,
        }
    }
    pub fn inner(&self) -> &anyhow::Error {
        match self {
            ConnectionError::AuthFailed(_, e) => e,
            ConnectionError::General(_, e) => e,
        }
    }
    pub fn connection_title(&self) -> Option<&str> {
        match self {
            ConnectionError::AuthFailed(title, _) => title.as_deref(),
            ConnectionError::General(title, _) => title.as_deref(),
        }
    }
}

pub type IconFactory<M> = Box<dyn Fn(&M) -> gtk::Widget>;

/// Specifies how this entity should be represented with an icon, if at all.
/// Any named or custom icon should have a width of 16px.
#[derive(Clone)]
pub enum IconSpec<M> {
    /// Use the default icon.
    Default,
    /// Do not use an icon.
    None,
    /// Use a named GTK icon.
    Named(Cow<'static, str>),
    /// Generate a custom GTK widget to be used as the widget. Callers MUST only try to use the
    /// returned widget if it doesn't already have a parent.
    Custom(Arc<IconFactory<M>>),
}

#[derive(Clone, Debug)]
pub enum PreferencesGroupOrPage {
    Group(adw::PreferencesGroup),
    Page(adw::PreferencesPage),
}

impl<M> fmt::Debug for IconSpec<M> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IconSpec::Default => fmt::Formatter::write_str(fmt, "Default"),
            IconSpec::None => fmt::Formatter::write_str(fmt, "None"),
            IconSpec::Named(named) => fmt::Formatter::debug_tuple(fmt, "Named")
                .field(named)
                .finish(),
            IconSpec::Custom(_) => fmt::Formatter::write_str(fmt, "Custom(...)"),
        }
    }
}

/// Metadata about a connection. Only the title is required.
///
/// On the builder type, build can be unwrapped as long as the title is set.
#[derive(Builder, Debug, Clone)]
#[builder(pattern = "owned")]
#[non_exhaustive]
pub struct ConnectionMetadata {
    pub title: String,
    #[builder(default = "None")]
    pub subtitle: Option<String>,
    #[builder(default = "IconSpec::Default")]
    pub icon: IconSpec<ConnectionMetadata>,
}

/// Metadata about a server. Only the title is required.
///
/// On the builder type, build can be unwrapped as long as the title is set.
#[derive(Builder, Debug, Clone)]
#[builder(pattern = "owned")]
#[non_exhaustive]
pub struct ServerMetadata {
    pub title: String,
    #[builder(default = "None")]
    pub subtitle: Option<String>,
    #[builder(default = "None")]
    pub is_online: Option<bool>,
    #[builder(default = "IconSpec::Default")]
    pub icon: IconSpec<ServerMetadata>,
}

pub trait FieldMonitorApplication {}

/// Constructor for ConnectionProvider and static members for ConnectionProviders.
/// This is separate to allow trait objects / dynamic dispatch.
pub trait ConnectionProviderConstructor: Send + Sync {
    /// Creates the provider.
    #[allow(clippy::new_ret_no_self, clippy::wrong_self_convention)]
    fn new(&self) -> Box<dyn ConnectionProvider>;
}

/// A provider for creating new connections. Each provider can create new connections
/// of a defined type.
pub trait ConnectionProvider {
    /// Tag for configuration connection. Will be serialized with configuration
    /// and used to match connection providers.
    fn tag(&self) -> &'static str;

    /// Title describing the provider.
    fn title(&self) -> Cow<'static, str>;

    /// Plural title of the provider. This may describe a group of multiple connections created
    /// by the provider.
    fn title_plural(&self) -> Cow<str>;

    /// Title to display when showing the dialog to add a new connection of this type.
    fn add_title(&self) -> Cow<str>;

    /// Title for a connection given it's config, or None if the config can not be resolved to a title.
    fn title_for<'a>(&self, config: &'a ConnectionConfiguration) -> Option<&'a str>;

    /// Returns a description to be shown in the "add new connection" dialog.
    fn description(&self) -> Cow<str>;

    /// Creates a preference page (or other applicable widget) for configuring a new connection.
    ///
    /// If this is for modifying an existing configuration, the current configuration is set.
    fn preferences(&self, configuration: Option<&ConnectionConfiguration>) -> gtk::Widget;

    /// Update a connection configuration from a configured preference page.
    /// It may be asserted that the  passed `preferences` are a widget returned from `preferences`.
    ///
    /// If an error is returned, the preferences will still be shown to the user. This means
    /// the implementation may modify the preferences to hint at input errors. It should not show
    /// the error message of the returned result, this is presented by the caller.
    fn update_connection(
        &self,
        preferences: gtk::Widget,
        configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>>;

    /// Creates a preference group or page for configuring credentials.
    ///
    /// This is shown to the user when a connection with the stored credentials
    /// could not be made, or when no credentials were stored.
    ///
    /// `server_path` may be set if the authentication failed while connecting to
    /// a specific server.
    ///
    /// The passed parameter contains the current configuration before.
    fn configure_credentials(
        &self,
        server_path: &[String],
        configuration: &ConnectionConfiguration,
    ) -> PreferencesGroupOrPage;

    /// Update the credentials of a connection.
    /// It may be asserted that the  passed `preferences` are a widget returned from
    /// `configure_credentials`.
    ///
    /// `server_path` may be set if the configuration was made for a specific server.
    ///
    /// If an error is returned, the preferences will still be shown to the user. This means
    /// the implementation may modify the preferences to hint at input errors. It should not show
    /// the error message of the returned result, this is presented by the caller.
    fn store_credentials(
        &self,
        server_path: &[String],
        preferences: gtk::Widget,
        configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>>;

    /// Try to load a connection configuration into a connection.
    /// The tag inside the configuration must match [`Self::tag`], otherwise the method
    /// may error or incorrectly try to load.
    /// This returns a future that should resolve ASAP, since the UI may not be able to indicate
    /// the loading process.
    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<ConnectionResult<Box<dyn Connection>>>;
}

/// Parameters for an action. Can be downcast to expected type.
pub type Parameters = Box<dyn Any>;

/// A future that executes whatever action was requested. These actions are defined by `actions`
/// methods on `Connection` and `ServerConnection`. See also `ActionMap`, `ServerAction`.
/// Parameters: static parameters, parent window, toast overlay
/// Return value: True if the connection should be reloaded, false otherwise.
pub type ActionExecuteFut<'a> =
    dyn Fn(Parameters, Option<gtk::Window>, Option<adw::ToastOverlay>) -> LocalBoxFuture<'a, bool>;
pub type ServerMap = IndexMap<Cow<'static, str>, Box<dyn ServerConnection>>;
pub type ServerMapSend = IndexMap<Cow<'static, str>, Box<dyn ServerConnection + Send>>;

pub struct ServerAction<'a> {
    static_parameters: Parameters,
    action_fn: Box<ActionExecuteFut<'a>>,
}

impl<'a> ServerAction<'a> {
    pub fn new(static_parameters: Parameters, action_fn: Box<ActionExecuteFut<'a>>) -> Self {
        Self {
            static_parameters,
            action_fn,
        }
    }

    /// Execute the action.
    pub fn execute(
        self,
        window: Option<&gtk::Window>,
        toast_overlay: Option<&adw::ToastOverlay>,
    ) -> LocalBoxFuture<'a, bool> {
        (self.action_fn)(
            self.static_parameters,
            window.cloned(),
            toast_overlay.cloned(),
        )
    }
}

/// Something that various actions can be performed on.
/// Not to be confused with GTK's Actionable, although the concepts and purpose are similar.
pub trait Actionable {
    /// Get the list of supported action IDs and titles.
    fn actions(&self) -> Vec<(Cow<'static, str>, Cow<'static, str>)> {
        vec![]
    }

    /// Get an action, if supported; see `actions`.
    fn action<'a>(&self, _action_id: &str) -> Option<ServerAction<'a>> {
        None
    }
}

/// A connection. Represents one or more servers which are logically
/// grouped together.
///
/// It manages zero, one or multiple servers.
pub trait Connection: Actionable {
    /// Metadata about the connection.
    fn metadata(&self) -> ConnectionMetadata;

    /// Returns the servers managed by this connection.
    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>>;
}

/// A single instance of a server to connect to.
/// It may contain sub-servers.
pub trait ServerConnection: Actionable {
    /// Metadata about the server.
    fn metadata(&self) -> ServerMetadata;

    /// List of supported adapters that can be used to connect to the server as tuples (tag, human-readable name)
    fn supported_adapters(&self) -> Vec<(Cow<str>, Cow<str>)>;

    /// Create an adapter of the given type, if supported (see `supported_adapters`).
    /// If not supported, may fail or panic (panic only if `supported_adapters` can never return
    /// that adapter).
    fn create_adapter(&self, tag: &str) -> LocalBoxFuture<ConnectionResult<Box<dyn Adapter>>>;

    /// Returns the sub-servers grouped under this server (if any).
    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move { Ok(IndexMap::new()) })
    }
}
