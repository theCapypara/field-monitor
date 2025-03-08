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
use crate::application::FieldMonitorApplication;
use crate::connection_loader::ConnectionLoader;
use crate::quick_connect::preferences::QuickConnectPreferences;
use crate::remote_server_info::RemoteServerInfo;
use anyhow::anyhow;
use fluent_uri::Uri;
use futures::future::LocalBoxFuture;
use futures::TryFutureExt;
use gettextrs::gettext;
use glib::prelude::*;
use gtk::Widget;
use libfieldmonitor::adapter::rdp::RdpAdapter;
use libfieldmonitor::adapter::spice::SpiceAdapter;
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::connection::{
    Actionable, ConfigAccess, ConfigAccessMut, Connection, ConnectionConfiguration,
    ConnectionError, ConnectionInstance, ConnectionMetadata, ConnectionMetadataBuilder,
    ConnectionProvider, ConnectionResult, DualScopedConnectionConfiguration, IconSpec,
    PreferencesGroupOrPage, ServerConnection, ServerMap, ServerMetadata, ServerMetadataBuilder,
};
use libfieldmonitor::{ManagesSecrets, NullSecretsManager};
use secure_string::SecureString;
use serde::Deserialize;
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::io::{Read, Seek};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

mod preferences;
mod rdp;
mod spice;
mod util;
mod virt_viewer_file;
mod vnc;

thread_local! {
    static PROVIDER: OnceCell<Rc<Box<dyn ConnectionProvider>>> = OnceCell::new();
}
static NULL_SECRETS_MANAGER: OnceLock<Arc<Box<dyn ManagesSecrets>>> = OnceLock::new();
const QUICK_CONNECT_TAG: &str = "__qc__";
const QUICK_CONNECT_SERVER_TAG: &str = "srv";
type OptionResult<T> = Result<T, ()>;

pub async fn try_from_uri(
    uri: &str,
    app: &FieldMonitorApplication,
    window: Option<&impl IsA<gtk::Window>>,
) -> Option<ConnectionResult<RemoteServerInfo<'static>>> {
    // RDP "URIs" are not actually valid URIs.
    let uri_lower = uri.to_lowercase();
    if uri_lower.starts_with("rdp://") {
        rdp::try_from_uri(&uri_lower, app, window).await.ok()
    } else if let Ok(uri) = Uri::parse(uri) {
        vnc::try_from_uri(&uri, app, window)
            .or_else(|_| spice::try_from_uri(&uri, app, window))
            .await
            .ok()
    } else {
        None
    }
}

pub async fn try_from_file(
    stream: impl Read + Seek,
    app: &FieldMonitorApplication,
    window: Option<&impl IsA<gtk::Window>>,
) -> Option<ConnectionResult<RemoteServerInfo<'static>>> {
    rdp::try_from_file(stream, app, window)
        .or_else(|mut stream| {
            stream.rewind().ok();
            virt_viewer_file::try_from_file(stream, app, window)
        })
        .await
        .ok()
}

async fn construct<F>(
    app: &FieldMonitorApplication,
    window: Option<&impl IsA<gtk::Window>>,
    propagate_config: F,
) -> ConnectionResult<RemoteServerInfo<'static>>
where
    F: Fn(&mut ConnectionConfiguration) -> ConnectionResult<()>,
{
    let mut config = ConnectionConfiguration::new(
        format!("{QUICK_CONNECT_TAG}-{}", Uuid::now_v7()),
        QUICK_CONNECT_TAG.to_string(),
        NULL_SECRETS_MANAGER
            .get_or_init(|| Arc::new(Box::new(NullSecretsManager)))
            .clone(),
    );
    propagate_config(&mut config)?;
    if config.title().is_empty() {
        config.set_title(&gettext("Untitled Server"));
    }

    let connection_id = config.id().to_string();
    let adapter = config.adapter();
    let path = format!("{connection_id}/{QUICK_CONNECT_SERVER_TAG}");

    let instance = new_qc_instance(config).await;
    let mut connections = HashMap::with_capacity(1);
    connections.insert(connection_id, instance);

    let Some(loader) = ConnectionLoader::load_server(
        Some(&connections),
        window.map(Cast::upcast_ref),
        &path,
        Some(app.clone()),
    )
    .await
    else {
        return Err(ConnectionError::General(
            None,
            anyhow!("connection loader failed"),
        ));
    };

    Ok(RemoteServerInfo::new(
        path.into(),
        adapter.tag().into(),
        loader.server_title().await.into(),
        loader.connection_title().await.into(),
        loader,
    ))
}

async fn new_qc_instance(config: ConnectionConfiguration) -> ConnectionInstance {
    PROVIDER
        .with(|provider| {
            ConnectionInstance::new(
                DualScopedConnectionConfiguration::new_unified(config),
                provider
                    .get_or_init(|| Rc::new(Box::new(QuickConnectConnectionProvider::new())))
                    .clone(),
            )
        })
        .await
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum QuickConnectAdapterType {
    Spice,
    Rdp,
    Vnc,
}

impl QuickConnectAdapterType {
    pub fn tag(&self) -> &'static str {
        match self {
            QuickConnectAdapterType::Spice => SpiceAdapter::TAG,
            QuickConnectAdapterType::Rdp => RdpAdapter::TAG,
            QuickConnectAdapterType::Vnc => VncAdapter::TAG,
        }
    }
}

struct QuickConnectConnectionProvider;

impl QuickConnectConnectionProvider {
    fn new() -> Self {
        Self
    }
}

impl ConnectionProvider for QuickConnectConnectionProvider {
    fn tag(&self) -> &'static str {
        QUICK_CONNECT_TAG
    }

    fn title(&self) -> Cow<'static, str> {
        gettext("Quick Connect").into()
    }

    fn title_plural(&self) -> Cow<str> {
        Cow::Borrowed("")
    }

    fn add_title(&self) -> Cow<str> {
        Cow::Borrowed("")
    }

    fn title_for<'a>(&self, config: &'a ConnectionConfiguration) -> Option<&'a str> {
        Some(config.title())
    }

    fn description(&self) -> Cow<str> {
        Cow::Borrowed("")
    }

    fn icon(&self) -> IconSpec<()> {
        IconSpec::Default
    }

    fn preferences(&self, _configuration: Option<&ConnectionConfiguration>) -> Widget {
        // This should never be called, this kind of connection is never actually added to the app,
        // only used transiently.
        gtk::Label::new(Some("This connection can not be configured.")).upcast()
    }

    fn update_connection(
        &self,
        _preferences: Widget,
        configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async move { Ok(configuration) })
    }

    fn configure_credentials(
        &self,
        _server_path: &[String],
        configuration: &ConnectionConfiguration,
    ) -> PreferencesGroupOrPage {
        // This may be called if authentication fails, even if the connection can normally not
        // be configured.
        PreferencesGroupOrPage::Group(QuickConnectPreferences::new(Some(configuration)).upcast())
    }

    fn store_credentials(
        &self,
        _server_path: &[String],
        preferences: Widget,
        mut configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async move {
            let preferences = preferences
                .downcast::<QuickConnectPreferences>()
                .expect("store_credentials got invalid widget type");

            configuration = configuration.transform_update_separate(
                |c_session| {
                    c_session.set_user(&preferences.user());
                    c_session
                        .set_password(SecureString::from_str(&preferences.password()).unwrap());
                    Result::<(), Infallible>::Ok(())
                },
                |_| {
                    // Quick connect is never persisted.
                    Result::<(), Infallible>::Ok(())
                },
            )?;
            Ok(configuration)
        })
    }

    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<ConnectionResult<Box<dyn Connection>>> {
        Box::pin(async move {
            let con = QuickConnectServerConnection::new(configuration);
            let conbx: Box<dyn Connection> = Box::new(con);
            Ok(conbx)
        })
    }
}

#[derive(Clone)]
struct QuickConnectServerConnection {
    config: Arc<ConnectionConfiguration>,
}

impl QuickConnectServerConnection {
    pub fn adapter_tag(&self) -> Cow<'static, str> {
        self.config.adapter().tag().into()
    }

    pub fn adapter_label(&self) -> Cow<'static, str> {
        match self.config.adapter() {
            QuickConnectAdapterType::Spice => SpiceAdapter::label(),
            QuickConnectAdapterType::Rdp => RdpAdapter::label(),
            QuickConnectAdapterType::Vnc => VncAdapter::label(),
        }
    }
}

impl QuickConnectServerConnection {
    fn new(in_config: ConnectionConfiguration) -> Self {
        Self {
            config: in_config.into(),
        }
    }
}

impl Actionable for QuickConnectServerConnection {}

impl Connection for QuickConnectServerConnection {
    fn metadata(&self) -> LocalBoxFuture<ConnectionMetadata> {
        Box::pin(async move {
            ConnectionMetadataBuilder::default()
                .title(gettext("via Quick Connect"))
                .build()
                .unwrap()
        })
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            let mut map = ServerMap::new();
            map.insert(QUICK_CONNECT_SERVER_TAG.into(), Box::new(self.clone()));
            Ok(map)
        })
    }
}

impl ServerConnection for QuickConnectServerConnection {
    fn metadata(&self) -> LocalBoxFuture<ServerMetadata> {
        Box::pin(async move {
            ServerMetadataBuilder::default()
                .title(self.config.title().to_string())
                .build()
                .unwrap()
        })
    }

    fn supported_adapters(&self) -> LocalBoxFuture<Vec<(Cow<str>, Cow<str>)>> {
        Box::pin(async move { vec![(self.adapter_tag(), self.adapter_label())] })
    }

    fn create_adapter(&self, tag: &str) -> LocalBoxFuture<ConnectionResult<Box<dyn Adapter>>> {
        let tag = tag.to_string();
        Box::pin(async move {
            if tag != self.adapter_tag() {
                return Err(ConnectionError::General(None, anyhow!("unsupported tag")));
            }

            let bx: Box<dyn Adapter> = {
                match self.config.adapter() {
                    QuickConnectAdapterType::Spice => spice::make_adapter(&self.config),
                    QuickConnectAdapterType::Rdp => rdp::make_adapter(&self.config),
                    QuickConnectAdapterType::Vnc => vnc::make_adapter(&self.config),
                }
            };
            Ok(bx)
        })
    }
}

trait QuickConnectConfig {
    fn adapter(&self) -> QuickConnectAdapterType;
    fn set_adapter(&mut self, value: QuickConnectAdapterType);
    fn title(&self) -> &str;
    fn set_title(&mut self, value: &str);
    fn host(&self) -> &str;
    fn set_host(&mut self, value: &str);
    fn user(&self) -> Option<&str>;
    fn set_user(&mut self, value: &str);
    fn port(&self) -> Option<NonZeroU32>;
    fn set_port(&mut self, value: NonZeroU32);
    fn password(&self) -> Option<SecureString>;
    fn set_password(&mut self, value: SecureString);
}

impl QuickConnectConfig for ConnectionConfiguration {
    fn adapter(&self) -> QuickConnectAdapterType {
        match self.get_try_as_str("adapter") {
            Some("rdp") => QuickConnectAdapterType::Rdp,
            Some("vnc") => QuickConnectAdapterType::Vnc,
            _ => QuickConnectAdapterType::Spice,
        }
    }

    fn set_adapter(&mut self, value: QuickConnectAdapterType) {
        self.set_value(
            "adapter",
            match value {
                QuickConnectAdapterType::Spice => "spice",
                QuickConnectAdapterType::Rdp => "rdp",
                QuickConnectAdapterType::Vnc => "vnc",
            },
        )
    }

    fn title(&self) -> &str {
        self.get_try_as_str("title").unwrap_or_default()
    }

    fn set_title(&mut self, value: &str) {
        self.set_value("title", value);
    }

    fn host(&self) -> &str {
        self.get_try_as_str("host").unwrap_or_default()
    }

    fn set_host(&mut self, value: &str) {
        self.set_value("host", value);
    }

    fn user(&self) -> Option<&str> {
        self.get_try_as_str("user")
    }

    fn set_user(&mut self, value: &str) {
        self.set_value("user", value);
    }

    fn port(&self) -> Option<NonZeroU32> {
        self.get_try_as_u64("port").and_then(|v| {
            if v <= (u32::MAX as u64) {
                NonZeroU32::new(v as u32)
            } else {
                None
            }
        })
    }

    fn set_port(&mut self, value: NonZeroU32) {
        self.set_value("port", value.get());
    }

    fn password(&self) -> Option<SecureString> {
        self.get_try_as_sec_string("__password")
    }

    fn set_password(&mut self, value: SecureString) {
        self.set_secure_string("__password", value)
    }
}
