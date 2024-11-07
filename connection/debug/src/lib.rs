use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use adw::prelude::AdwDialogExt;
use anyhow::anyhow;
use async_std::task::sleep;
use futures::future::LocalBoxFuture;
use gtk::prelude::*;
use indexmap::IndexMap;
use log::debug;
use rand::{thread_rng, Rng};

use libfieldmonitor::adapter::rdp::RdpAdapter;
use libfieldmonitor::adapter::spice::SpiceAdapter;
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::connection::*;

use crate::arbitrary_adapter::DebugArbitraryAdapter;
use crate::behaviour_preferences::{DebugBehaviour, DebugBehaviourPreferences};
use crate::preferences::{DebugConfiguration, DebugMode, DebugPreferences};
use crate::vte_adapter::DebugVteAdapter;

mod arbitrary_adapter;
mod behaviour_preferences;
mod preferences;
mod vte_adapter;

const ICON: &str = "bug-symbolic";

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
        Cow::Borrowed("Debug")
    }

    fn title_plural(&self) -> Cow<str> {
        Cow::Borrowed("Debug Connections")
    }

    fn add_title(&self) -> Cow<str> {
        Cow::Borrowed("Add Debug Connection")
    }

    fn title_for<'a>(&self, config: &'a ConnectionConfiguration) -> Option<&'a str> {
        let title = config.title();
        if title.is_empty() {
            None
        } else {
            Some(title)
        }
    }

    fn description(&self) -> Cow<str> {
        Cow::Borrowed("Debug Connection")
    }

    fn icon(&self) -> IconSpec<()> {
        IconSpec::Named(ICON.into())
    }

    fn preferences(&self, configuration: Option<&ConnectionConfiguration>) -> gtk::Widget {
        DebugPreferences::new(configuration).upcast()
    }

    fn update_connection(
        &self,
        preferences: gtk::Widget,
        mut configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async {
            sleep(Duration::from_millis(thread_rng().gen_range(100..1200))).await;

            let preferences = preferences
                .downcast::<DebugPreferences>()
                .expect("update_connection got invalid widget type");

            configuration = configuration
                .transform_update_unified(|configuration| {
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
                    configuration.set_spice_host(&preferences.spice_host());
                    configuration.set_spice_password(&preferences.spice_password());
                    configuration.set_vte_adapter_enable(preferences.vte_adapter_enable());
                    configuration.set_custom_adapter_enable(preferences.custom_adapter_enable());
                    configuration.set_custom_overlayed(preferences.custom_overlayed());
                    Result::<(), Infallible>::Ok(())
                })
                .unwrap();

            // Update credentials
            let credentials = preferences.behaviour();
            self.store_credentials(&[], credentials.clone().upcast(), configuration)
                .await
        })
    }

    fn configure_credentials(
        &self,
        server_path: &[String],
        configuration: &ConnectionConfiguration,
    ) -> PreferencesGroupOrPage {
        debug!("configure_credentials server_path : {server_path:?}");
        PreferencesGroupOrPage::Group(DebugBehaviourPreferences::new(Some(configuration)).upcast())
    }

    fn store_credentials(
        &self,
        server_path: &[String],
        preferences: gtk::Widget,
        mut configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        debug!("store_credentials server_path : {server_path:?}");
        Box::pin(async move {
            sleep(Duration::from_millis(thread_rng().gen_range(100..400))).await;

            let preferences = preferences
                .downcast::<DebugBehaviourPreferences>()
                .expect("store_credentials got invalid widget type");

            configuration = configuration.transform_update_separate(
                |c_session| {
                    c_session.set_load_servers_behaviour(preferences.load_servers_behaviour());
                    c_session.set_connect_behaviour(preferences.connect_behaviour());

                    c_session.set_store_session(&preferences.store_session());
                    Result::<(), Infallible>::Ok(())
                },
                |c_persistent| {
                    c_persistent.set_load_servers_behaviour(preferences.load_servers_behaviour());
                    c_persistent.set_connect_behaviour(preferences.connect_behaviour());

                    c_persistent.set_store_persistent(&preferences.store_persistent());
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
            sleep(Duration::from_millis(thread_rng().gen_range(100..1200))).await;

            let title = configuration.title().to_string();

            let subtitle = match configuration.mode() {
                DebugMode::Single => None,
                DebugMode::Multi => Some("multi mode".to_string()),
                DebugMode::Complex => Some("complex mode".to_string()),
                DebugMode::NoServers => Some("no servers".to_string()),
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

impl Actionable for DebugConnection {
    fn actions(&self) -> Vec<(Cow<'static, str>, Cow<'static, str>)> {
        match self.config.mode() {
            DebugMode::Complex => {
                vec![
                    (
                        Cow::Borrowed("dialog_persisted"),
                        Cow::Borrowed("Show dialog: Persisted value"),
                    ),
                    (
                        Cow::Borrowed("dialog_session"),
                        Cow::Borrowed("Show dialog: Session value"),
                    ),
                    (Cow::Borrowed("bazbaz"), Cow::Borrowed("Show toast")),
                ]
            }
            _ => vec![],
        }
    }

    fn action<'a>(&self, action_id: &str) -> Option<ServerAction<'a>> {
        match action_id {
            "dialog_persisted" => Some(ServerAction::new(
                Box::new(self.config.store_persistent().to_string()),
                Box::new(|params, window, _toasts| {
                    Box::pin(async move {
                        let persisted_v = params.downcast::<String>().unwrap();
                        adw::AlertDialog::builder()
                            .body(&*persisted_v)
                            .build()
                            .present(window.as_ref());
                        true
                    })
                }),
            )),

            "dialog_session" => Some(ServerAction::new(
                Box::new(self.config.store_session().to_string()),
                Box::new(|params, window, _toasts| {
                    Box::pin(async move {
                        let session_v = params.downcast::<String>().unwrap();
                        adw::AlertDialog::builder()
                            .body(&*session_v)
                            .build()
                            .present(window.as_ref());
                        true
                    })
                }),
            )),

            "bazbaz" => Some(ServerAction::new(
                Box::new(()),
                Box::new(|_params, _window, toasts| {
                    Box::pin(async move {
                        sleep(Duration::from_secs(2)).await;
                        let toast = adw::Toast::builder().title("Foobar").timeout(5).build();
                        if let Some(toasts) = toasts {
                            toasts.add_toast(toast);
                        }
                        false
                    })
                }),
            )),
            _ => None,
        }
    }
}

impl Connection for DebugConnection {
    fn metadata(&self) -> ConnectionMetadata {
        ConnectionMetadataBuilder::default()
            .title(self.title.clone())
            .subtitle(self.subtitle.clone())
            .icon(IconSpec::Named(ICON.into()))
            .build()
            .unwrap()
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
                                    ServerMetadataBuilder::default()
                                        .title("Debug Server".to_string())
                                        .build()
                                        .unwrap(),
                                    self.config.clone(),
                                )),
                            );
                        }
                        DebugMode::Multi => {
                            hm.insert(
                                "server1".into(),
                                Box::new(DebugConnectionServer::new(
                                    ServerMetadataBuilder::default()
                                        .title("Debug Server".to_string())
                                        .subtitle(Some("Is the first server".to_string()))
                                        .build()
                                        .unwrap(),
                                    self.config.clone(),
                                )),
                            );
                            hm.insert(
                                "server2".into(),
                                Box::new(DebugConnectionServer::new(
                                    ServerMetadataBuilder::default()
                                        .title("Server 2".to_string())
                                        .subtitle(Some("Has no icon".to_string()))
                                        .icon(IconSpec::None)
                                        .build()
                                        .unwrap(),
                                    self.config.clone(),
                                )),
                            );
                            hm.insert(
                                "server3".into(),
                                Box::new(DebugConnectionServer::new(
                                    ServerMetadataBuilder::default()
                                        .title("Server 3".to_string())
                                        .subtitle(Some("This is marked as offline".to_string()))
                                        .is_online(Some(false))
                                        .build()
                                        .unwrap(),
                                    self.config.clone(),
                                )),
                            );
                        }
                        DebugMode::Complex => {
                            let mut root1 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("Root 1".to_string())
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let mut r1_level1_1 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R1 L1_1".to_string())
                                    .subtitle(Some("Has no icon".to_string()))
                                    .icon(IconSpec::None)
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let mut r1_level1_2 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R1 L1_2".to_string())
                                    .subtitle(Some("Is online".to_string()))
                                    .is_online(Some(true))
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let r1_level2_1 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R1 L1_2 L2_1".to_string())
                                    .subtitle(Some("Is offline".to_string()))
                                    .is_online(Some(false))
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            r1_level1_2.add_server(r1_level2_1);
                            r1_level1_1.no_adapters();
                            r1_level1_2.expose_dummy_actions();
                            root1.add_server(r1_level1_1);
                            root1.add_server(r1_level1_2);
                            root1.expose_dummy_actions();

                            let mut root2 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("Root 2".to_string())
                                    .subtitle(Some("Is online".to_string()))
                                    .is_online(Some(true))
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let mut r2_level1 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R2 1".to_string())
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let mut r2_level2 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R2 2".to_string())
                                    .subtitle(Some("has a named icon".to_string()))
                                    .icon(IconSpec::Named(Cow::Borrowed("go-home-symbolic")))
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let mut r2_level3 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R2 3".to_string())
                                    .subtitle(Some("has a custom widget icon".to_string()))
                                    .icon(IconSpec::Custom(Arc::new(Box::new(|_meta| {
                                        gtk::Spinner::builder().spinning(true).build().upcast()
                                    }))))
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let r2_level4 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R2 4".to_string())
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            r2_level3.add_server(r2_level4);
                            r2_level2.add_server(r2_level3);
                            r2_level1.add_server(r2_level2);
                            r2_level1.no_adapters();
                            root2.add_server(r2_level1);

                            hm.insert("server1".into(), Box::new(root1));
                            hm.insert("server2".into(), Box::new(root2));
                        }
                        DebugMode::NoServers => {}
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
    metadata: ServerMetadata,
    config: ConnectionConfiguration,
    servers: HashMap<Cow<'static, str>, DebugConnectionServer>,
    has_adapters: bool,
    has_actions: bool,
}

impl DebugConnectionServer {
    fn new(metadata: ServerMetadata, config: ConnectionConfiguration) -> Self {
        Self {
            metadata,
            config,
            servers: HashMap::new(),
            has_adapters: true,
            has_actions: false,
        }
    }

    fn add_server(&mut self, server: DebugConnectionServer) {
        self.servers
            .insert(Cow::Owned(server.metadata.title.clone()), server);
    }

    fn no_adapters(&mut self) {
        self.has_adapters = false;
    }

    fn expose_dummy_actions(&mut self) {
        self.has_actions = true;
    }
}

impl Actionable for DebugConnectionServer {
    fn actions(&self) -> Vec<(Cow<'static, str>, Cow<'static, str>)> {
        vec![
            (Cow::Borrowed("foobar"), Cow::Borrowed("Show dialog")),
            (Cow::Borrowed("bazbaz"), Cow::Borrowed("Show toast")),
        ]
    }

    fn action<'a>(&self, action_id: &str) -> Option<ServerAction<'a>> {
        match action_id {
            "foobar" => Some(ServerAction::new(
                Box::new(()),
                Box::new(|_params, window, _toasts| {
                    Box::pin(async move {
                        adw::AlertDialog::builder()
                            .body("Testing! This is from a server.")
                            .build()
                            .present(window.as_ref());
                        true
                    })
                }),
            )),
            "bazbaz" => Some(ServerAction::new(
                Box::new(()),
                Box::new(|_params, _window, toasts| {
                    Box::pin(async move {
                        sleep(Duration::from_secs(2)).await;
                        let toast = adw::Toast::builder()
                            .title("Server bazbaz")
                            .timeout(5)
                            .build();
                        if let Some(toasts) = toasts {
                            toasts.add_toast(toast);
                        }
                        false
                    })
                }),
            )),
            _ => None,
        }
    }
}

impl ServerConnection for DebugConnectionServer {
    fn metadata(&self) -> ServerMetadata {
        self.metadata.clone()
    }

    fn supported_adapters(&self) -> Vec<(Cow<str>, Cow<str>)> {
        if !self.has_adapters {
            return vec![];
        }
        let mut adapters = Vec::with_capacity(4);
        if self.config.vnc_adapter_enable() {
            adapters.push((VncAdapter::TAG.into(), VncAdapter::label()));
        }
        if self.config.rdp_adapter_enable() {
            adapters.push((RdpAdapter::TAG.into(), RdpAdapter::label()));
        }
        if self.config.spice_adapter_enable() {
            adapters.push((SpiceAdapter::TAG.into(), SpiceAdapter::label()));
        }
        if self.config.vte_adapter_enable() {
            adapters.push((DebugVteAdapter::TAG.into(), "VTE".into()));
        }
        if self.config.custom_adapter_enable() {
            adapters.push((DebugArbitraryAdapter::TAG.into(), "Arbitrary Widget".into()));
        }
        adapters
    }

    fn create_adapter(
        &self,
        tag: &str,
    ) -> LocalBoxFuture<Result<Box<dyn Adapter>, ConnectionError>> {
        let tag = tag.to_string();
        Box::pin(async move {
            match self.config.connect_behaviour() {
                DebugBehaviour::Ok => {
                    let adapter: Box<dyn Adapter> = match &*tag {
                        VncAdapter::TAG => {
                            let (host, port) = parse_host_port(self.config.vnc_host())?;
                            Box::new(VncAdapter::new(
                                host.to_string(),
                                port,
                                self.config.vnc_user().to_string(),
                                self.config.vnc_password().into(),
                            ))
                        }
                        RdpAdapter::TAG => {
                            let (host, port) = parse_host_port(self.config.rdp_host())?;
                            Box::new(RdpAdapter::new(
                                host.to_string(),
                                port,
                                self.config.rdp_user().to_string(),
                                self.config.rdp_password().into(),
                            ))
                        }
                        SpiceAdapter::TAG => {
                            let (host, port) = parse_host_port(self.config.spice_host())?;
                            Box::new(SpiceAdapter::new(
                                host.to_string(),
                                port,
                                "".to_string(),
                                self.config.spice_password().into(),
                            ))
                        }
                        DebugVteAdapter::TAG => Box::new(DebugVteAdapter {
                            mode: self.config.connect_behaviour(),
                        }),
                        DebugArbitraryAdapter::TAG => Box::new(DebugArbitraryAdapter {
                            mode: self.config.connect_behaviour(),
                            overlayed: self.config.custom_overlayed(),
                        }),
                        _ => unimplemented!("invalid tag"),
                    };

                    Ok(adapter)
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

fn parse_host_port(host: &str) -> ConnectionResult<(&str, u32)> {
    let mut host_parts = host.split(":");
    let Some(host) = host_parts.next() else {
        return Err(ConnectionError::General(
            Some("invalid host".to_string()),
            anyhow!("invalid host"),
        ));
    };
    let Some(port) = host_parts.next() else {
        return Err(ConnectionError::General(
            Some("invalid host".to_string()),
            anyhow!("invalid host"),
        ));
    };
    let port: u32 = match port.parse() {
        Ok(v) => v,
        Err(e) => {
            return Err(ConnectionError::General(
                Some("invalid port".to_string()),
                e.into(),
            ));
        }
    };
    Ok((host, port))
}
