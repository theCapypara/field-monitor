use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

use anyhow::anyhow;
use async_std::task::sleep;
use futures::future::LocalBoxFuture;
use gtk::prelude::*;
use indexmap::IndexMap;
use rand::{Rng, thread_rng};

use libfieldmonitor::adapter::rdp::RdpAdapter;
use libfieldmonitor::adapter::spice::SpiceAdapter;
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::connection::{
    Connection, ConnectionConfiguration, ConnectionError, ConnectionMetadata, ConnectionProvider,
    ConnectionProviderConstructor, ConnectionResult, ServerConnection, ServerMap, ServerMetadata,
};

use crate::behaviour_preferences::{DebugBehaviour, DebugBehaviourPreferences};
use crate::preferences::{DebugConfiguration, DebugMode, DebugPreferences};
use crate::vte_adapter::DebugVteAdapter;

mod behaviour_preferences;
mod preferences;
mod vte_adapter;

pub struct DebugConnectionProviderConstructor;

impl ConnectionProviderConstructor for DebugConnectionProviderConstructor {
    fn new(&self) -> Box<dyn ConnectionProvider> {
        Box::new(DebugConnectionProvider {})
    }
}

pub struct DebugConnectionProvider {}

impl ConnectionProvider for DebugConnectionProvider {
    fn tag(&self) -> &'static str {
        "debug"
    }

    fn title(&self) -> Cow<'static, str> {
        Cow::Borrowed("Debug Connection")
    }

    fn title_plural(&self) -> Cow<str> {
        Cow::Borrowed("Debug Connections")
    }

    fn add_title(&self) -> Cow<str> {
        Cow::Borrowed("Add Debug Connection")
    }

    fn description(&self) -> Cow<str> {
        Cow::Borrowed("Debug Connection")
    }

    fn preferences(&self, configuration: Option<&ConnectionConfiguration>) -> gtk::Widget {
        DebugPreferences::new(configuration).upcast()
    }

    fn update_connection(
        &self,
        preferences: gtk::Widget,
        mut configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<ConnectionConfiguration>> {
        Box::pin(async {
            sleep(Duration::from_millis(thread_rng().gen_range(100..1200))).await;

            let preferences = preferences
                .downcast::<DebugPreferences>()
                .expect("update_connection got invalid widget type");

            // Update general config
            configuration.set_title(&preferences.title());
            configuration.set_mode(preferences.mode());
            configuration.set_vnc_adapter_enable(preferences.vnc_adapter_enable());
            configuration.set_vnc_host(&preferences.vnc_host());
            configuration.set_vnc_user(&preferences.vnc_user());
            configuration.set_vnc_password(&preferences.vnc_password());
            configuration.set_rdp_adapter_enable(preferences.rdp_adapter_enable());
            configuration.set_rdp_host(&preferences.rdp_host());
            configuration.set_rdp_user(&preferences.rdp_user());
            configuration.set_rdp_password(&preferences.rdp_password());
            configuration.set_spice_adapter_enable(preferences.spice_adapter_enable());
            configuration.set_vte_adapter_enable(preferences.vte_adapter_enable());

            // Update credentials
            let credentials = preferences.behaviour();
            self.store_credentials(credentials.clone().upcast(), configuration)
                .await
        })
    }

    fn configure_credentials(&self, configuration: &ConnectionConfiguration) -> gtk::Widget {
        DebugBehaviourPreferences::new(Some(configuration)).upcast()
    }

    fn store_credentials(
        &self,
        preferences: gtk::Widget,
        mut configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<ConnectionConfiguration>> {
        Box::pin(async move {
            sleep(Duration::from_millis(thread_rng().gen_range(100..400))).await;

            let preferences = preferences
                .downcast::<DebugBehaviourPreferences>()
                .expect("store_credentials got invalid widget type");

            configuration.set_load_servers_behaviour(preferences.load_servers_behaviour());
            configuration.set_connect_behaviour(preferences.connect_behaviour());
            Ok(configuration)
        })
    }

    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<ConnectionResult<Box<dyn Connection>>> {
        Box::pin(async move {
            sleep(Duration::from_millis(thread_rng().gen_range(100..1200))).await;

            let title = configuration.title().to_string();

            let subtitle = match configuration.mode() {
                DebugMode::Single => None,
                DebugMode::Multi => Some("multi mode".to_string()),
                DebugMode::Complex => Some("complex mode".to_string()),
            };

            let c: Box<dyn Connection> =
                Box::new(DebugConnection::new(title, subtitle, configuration));
            Ok(c)
        })
    }
}

#[derive(Clone)]
pub struct DebugConnection {
    title: String,
    subtitle: Option<String>,
    config: ConnectionConfiguration,
}

impl Connection for DebugConnection {
    fn metadata(&self) -> ConnectionMetadata {
        ConnectionMetadata {
            title: self.title.clone(),
            subtitle: self.subtitle.clone(),
        }
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            sleep(Duration::from_millis(thread_rng().gen_range(100..1200))).await;

            match self.config.load_servers_behaviour() {
                DebugBehaviour::Ok => {
                    let mut hm: IndexMap<Cow<_>, Box<dyn ServerConnection>> = IndexMap::new();

                    match self.config.mode() {
                        DebugMode::Single => {
                            hm.insert(
                                "server1".into(),
                                Box::new(DebugConnectionServer::new(
                                    "Debug Server",
                                    self.config.clone(),
                                )),
                            );
                        }
                        DebugMode::Multi => {
                            hm.insert(
                                "server1".into(),
                                Box::new(DebugConnectionServer::new(
                                    "Server 1",
                                    self.config.clone(),
                                )),
                            );
                            hm.insert(
                                "server2".into(),
                                Box::new(DebugConnectionServer::new(
                                    "Server 2",
                                    self.config.clone(),
                                )),
                            );
                            hm.insert(
                                "server3".into(),
                                Box::new(DebugConnectionServer::new(
                                    "Server 3",
                                    self.config.clone(),
                                )),
                            );
                        }
                        DebugMode::Complex => {
                            let mut root1 =
                                DebugConnectionServer::new("Root 1", self.config.clone());
                            let mut r1_level1_1 =
                                DebugConnectionServer::new("R1 L1_1", self.config.clone());
                            let mut r1_level1_2 =
                                DebugConnectionServer::new("R1 L1_2", self.config.clone());
                            let r1_level2_1 =
                                DebugConnectionServer::new("R1 L1_2 L2_1", self.config.clone());
                            r1_level1_2.add_server(r1_level2_1);
                            r1_level1_1.no_adapters();
                            r1_level1_2.no_adapters();
                            root1.add_server(r1_level1_1);
                            root1.add_server(r1_level1_2);

                            let mut root2 =
                                DebugConnectionServer::new("Root 2", self.config.clone());
                            let mut r2_level1 =
                                DebugConnectionServer::new("R2 1", self.config.clone());
                            let mut r2_level2 =
                                DebugConnectionServer::new("R2 2", self.config.clone());
                            let mut r2_level3 =
                                DebugConnectionServer::new("R2 3", self.config.clone());
                            let r2_level4 = DebugConnectionServer::new("R2 4", self.config.clone());
                            r2_level3.add_server(r2_level4);
                            r2_level2.add_server(r2_level3);
                            r2_level1.add_server(r2_level2);
                            r2_level1.no_adapters();
                            root2.add_server(r2_level1);
                            root2.set_subtitle(None);

                            hm.insert("server1".into(), Box::new(root1));
                            hm.insert("server2".into(), Box::new(root2));
                        }
                    }

                    Ok(hm)
                }
                DebugBehaviour::AuthError => Err(ConnectionError::AuthFailed(
                    Some("debug auth failure (servers)".to_string()),
                    anyhow!("debug auth failure (servers)"),
                )),
                DebugBehaviour::GeneralError => Err(ConnectionError::General(
                    Some("debug general failure (servers)".to_string()),
                    anyhow!("debug general failure (servers)"),
                )),
            }
        })
    }
}

impl DebugConnection {
    fn new(title: String, subtitle: Option<String>, config: ConnectionConfiguration) -> Self {
        Self {
            title,
            subtitle,
            config,
        }
    }
}

#[derive(Clone)]
pub struct DebugConnectionServer {
    title: &'static str,
    subtitle: Option<&'static str>,
    config: ConnectionConfiguration,
    servers: HashMap<Cow<'static, str>, DebugConnectionServer>,
    has_adapters: bool,
}

impl DebugConnectionServer {
    fn new(title: &'static str, config: ConnectionConfiguration) -> Self {
        Self {
            title,
            subtitle: Some("Debug Subtitle"),
            config,
            servers: HashMap::new(),
            has_adapters: true,
        }
    }

    fn set_subtitle(&mut self, v: Option<&'static str>) {
        self.subtitle = v;
    }

    fn add_server(&mut self, server: DebugConnectionServer) {
        self.servers.insert(Cow::Borrowed(server.title), server);
    }

    fn no_adapters(&mut self) {
        self.has_adapters = false;
    }
}

impl ServerConnection for DebugConnectionServer {
    fn metadata(&self) -> ServerMetadata {
        ServerMetadata {
            title: self.title.to_string(),
            subtitle: self.subtitle.map(ToString::to_string),
        }
    }

    fn supported_adapters(&self) -> Vec<(Cow<str>, Cow<str>)> {
        if !self.has_adapters {
            return vec![];
        }
        let mut adapters = Vec::with_capacity(4);
        if self.config.vnc_adapter_enable() {
            adapters.push((VncAdapter::TAG, VncAdapter::label()));
        }
        if self.config.rdp_adapter_enable() {
            adapters.push((RdpAdapter::TAG, RdpAdapter::label()));
        }
        if self.config.spice_adapter_enable() {
            adapters.push((SpiceAdapter::TAG, SpiceAdapter::label()));
        }
        if self.config.vte_adapter_enable() {
            adapters.push((DebugVteAdapter::TAG, "VTE".into()));
        }
        adapters
    }

    fn create_adapter(
        &self,
        tag: &str,
    ) -> LocalBoxFuture<Result<Box<dyn Adapter>, ConnectionError>> {
        Box::pin(async move { todo!() })
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            sleep(Duration::from_millis(thread_rng().gen_range(50..200))).await;

            let mut hm: IndexMap<Cow<_>, Box<dyn ServerConnection>> = IndexMap::new();

            for (name, server) in &self.servers {
                hm.insert(name.clone(), Box::new(server.clone()));
            }

            Ok(hm)
        })
    }
}
