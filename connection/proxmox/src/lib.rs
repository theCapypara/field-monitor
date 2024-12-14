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
use std::future::Future;
use std::mem;
use std::num::NonZeroU32;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use crate::credential_preferences::ProxmoxCredentialPreferences;
use crate::preferences::{ProxmoxConfiguration, ProxmoxPreferences};
use crate::tokiort::{run_on_tokio, tkruntime};
use adw::prelude::Cast;
use anyhow::anyhow;
use async_std::task::sleep;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use gtk::Widget;
use http::Uri;
use libfieldmonitor::adapter::spice::{SpiceAdapter, SpiceSessionConfigBuilder};
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::adapter::vte_pty::VtePtyAdapter;
use libfieldmonitor::connection::*;
use libfieldmonitor::libexec_path;
use log::{error, warn};
use proxmox_api::{
    NodeId, NodeStatus, ProxmoxApiClient, Spiceproxy, Termproxy, VmConsoleProxyType, VmId,
    VmStatus, VmType, Vncproxy,
};
use secure_string::SecureString;

mod credential_preferences;
mod preferences;
mod tokiort;

pub const PTY_DRIVER_BIN: &str = "de.capypara.FieldMonitor.PtyDrv.Proxmox";

pub struct ProxmoxConnectionProviderConstructor;

impl ConnectionProviderConstructor for ProxmoxConnectionProviderConstructor {
    fn new(&self) -> Box<dyn ConnectionProvider> {
        Box::new(ProxmoxConnectionProvider {})
    }
}

pub struct ProxmoxConnectionProvider {}

impl ConnectionProvider for ProxmoxConnectionProvider {
    fn tag(&self) -> &'static str {
        "proxmox"
    }

    fn title(&self) -> Cow<'static, str> {
        gettext("Proxmox").into()
    }

    fn title_plural(&self) -> Cow<str> {
        gettext("Proxmox").into()
    }

    fn add_title(&self) -> Cow<str> {
        gettext("Add Proxmox Connection").into()
    }

    fn title_for<'a>(&self, config: &'a ConnectionConfiguration) -> Option<&'a str> {
        config.title()
    }

    fn description(&self) -> Cow<str> {
        gettext("Proxmox hypervisor connection").into()
    }

    fn icon(&self) -> IconSpec<()> {
        IconSpec::Named("connection-proxmox-symbolic".into())
    }

    fn preferences(&self, configuration: Option<&ConnectionConfiguration>) -> Widget {
        ProxmoxPreferences::new(configuration).upcast()
    }

    fn update_connection(
        &self,
        preferences: Widget,
        mut configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async {
            let preferences = preferences
                .downcast::<ProxmoxPreferences>()
                .expect("update_connection got invalid widget type");

            // Update general config
            configuration = configuration
                .transform_update_unified(|config| preferences.apply_general_config(config))?;

            // Update credentials
            let credentials = preferences.credentials();
            self.store_credentials(&[], credentials.clone().upcast(), configuration)
                .await
        })
    }

    fn configure_credentials(
        &self,
        _server_path: &[String],
        configuration: &ConnectionConfiguration,
    ) -> PreferencesGroupOrPage {
        PreferencesGroupOrPage::Group(
            ProxmoxCredentialPreferences::new(Some(configuration), true).upcast(),
        )
    }

    fn store_credentials(
        &self,
        _server_path: &[String],
        preferences: Widget,
        configuration: DualScopedConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<DualScopedConnectionConfiguration>> {
        Box::pin(async move {
            let preferences = preferences
                .downcast::<ProxmoxCredentialPreferences>()
                .expect("store_credentials got invalid widget type");

            configuration.transform_update_separate(
                |c_session| preferences.apply_persistent_config(c_session),
                |c_persistent| preferences.apply_session_config(c_persistent),
            )
        })
    }

    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<ConnectionResult<Box<dyn Connection>>> {
        Box::pin(async move {
            let con: ProxmoxConnection =
                run_on_tokio(ProxmoxConnection::connect(configuration)).await?;
            let conbx: Box<dyn Connection> = Box::new(con);
            Ok(conbx)
        })
    }
}

struct ProxmoxConnection {
    connection_id: String,
    title: String,
    client: Arc<ProxmoxApiClient>,
}

impl ProxmoxConnection {
    async fn connect(config: ConnectionConfiguration) -> ConnectionResult<Self> {
        let authority = format!(
            "{}:{}",
            config.hostname().unwrap_or_default(),
            config.port().map(NonZeroU32::get).unwrap_or(8006)
        );

        let api_root = Uri::builder()
            .scheme("https")
            .authority(authority)
            .path_and_query("/api2/json")
            .build()
            .map_err(|err| {
                ConnectionError::General(
                    Some(gettext(
                        "Was unable to build a valid URL to connect to. Check your settings.",
                    )),
                    anyhow!(err),
                )
            })?;

        let pass = config
            .password_or_apikey()
            .await
            .map_err(|err| {
                ConnectionError::General(
                    Some(gettext(
                        "Failed to retrieve API Key or Password from secrets service.",
                    )),
                    anyhow!(err),
                )
            })?
            .unwrap_or_else(|| SecureString::from_str("").unwrap());

        let client = if config.use_apikey() {
            ProxmoxApiClient::connect_with_apikey(
                &api_root,
                config.tokenid().unwrap_or_default(),
                pass,
                config.ignore_ssl_cert_error(),
            )
            .await
            .map_err(map_proxmox_error)
        } else {
            ProxmoxApiClient::connect_with_ticket(
                &api_root,
                config.username().unwrap_or_default(),
                pass,
                config.ignore_ssl_cert_error(),
            )
            .await
            .map_err(map_proxmox_error)
        }?;

        Ok(Self {
            connection_id: config.id().to_string(),
            title: config.title().unwrap_or_default().to_string(),
            client: Arc::new(client),
        })
    }
}

impl Actionable for ProxmoxConnection {}

impl Connection for ProxmoxConnection {
    fn metadata(&self) -> ConnectionMetadata {
        ConnectionMetadataBuilder::default()
            .title(self.title.clone())
            .icon(IconSpec::Named("connection-proxmox-symbolic".into()))
            .build()
            .unwrap()
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            let connection_id = self.connection_id.clone();
            let client = self.client.clone();
            let map = run_on_tokio(async move {
                let mut server_map = ServerMapSend::default();

                for node in client.nodes().await.map_err(map_proxmox_error)? {
                    server_map.insert(
                        node.node.to_string().into(),
                        Box::new(ProxmoxNode {
                            client: client.clone(),
                            connection_id: connection_id.clone(),
                            id: node.node,
                            status: NodeStatus::Online,
                        }),
                    );
                }

                Ok(server_map)
            })
            .await?;

            // TODO: Is this actually safe?
            let map_cast: ServerMap = unsafe { mem::transmute(map) };

            Ok(map_cast)
        })
    }
}

struct ProxmoxNode {
    client: Arc<ProxmoxApiClient>,
    connection_id: String,
    id: NodeId,
    status: NodeStatus,
}

impl Actionable for ProxmoxNode {
    fn actions(&self) -> Vec<(Cow<'static, str>, Cow<'static, str>)> {
        if self.status != NodeStatus::Offline {
            vec![
                ("nodereboot".into(), gettext("Reboot").into()),
                ("nodeshutdown".into(), gettext("Shutdown").into()),
            ]
        } else {
            vec![]
        }
    }

    fn action<'a>(&self, action_id: &str) -> Option<ServerAction<'a>> {
        match action_id {
            "nodereboot" => Some(self.act_reboot()),
            "nodeshutdown" => Some(self.act_shutdown()),
            _ => None,
        }
    }
}

impl ProxmoxNode {
    fn params(&self) -> ExecParams {
        ExecParams {
            client: self.client.clone(),
            node_id: Some(self.id.clone()),
            vm_id: None,
            vm_type: None,
        }
    }

    fn act_reboot<'a>(&self) -> ServerAction<'a> {
        ServerAction::new(
            Box::new(self.params()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let params = params.downcast::<ExecParams>().unwrap();

                    let (success, force_reload) = exec_cmd(
                        params,
                        |params| async move {
                            params.client.node_reboot(&params.node_id.unwrap()).await
                        },
                        || gettext("Reboot command successfully sent to server."),
                        |err| {
                            warn!("failed reboot: {err:?}");
                            gettext("Failed to send reboot command.")
                        },
                        toov.as_ref(),
                    )
                    .await;
                    success || force_reload
                })
            }),
        )
    }

    fn act_shutdown<'a>(&self) -> ServerAction<'a> {
        ServerAction::new(
            Box::new(self.params()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let params = params.downcast::<ExecParams>().unwrap();

                    let (success, force_reload) = exec_cmd(
                        params,
                        |params| async move {
                            params.client.node_shutdown(&params.node_id.unwrap()).await
                        },
                        || gettext("Shutdown command successfully sent to server."),
                        |err| {
                            warn!("failed shutdown: {err:?}");
                            gettext("Failed to send shutdown command.")
                        },
                        toov.as_ref(),
                    )
                    .await;
                    success || force_reload
                })
            }),
        )
    }
}

impl ServerConnection for ProxmoxNode {
    fn metadata(&self) -> ServerMetadata {
        let is_online = match self.status {
            NodeStatus::Online => Some(true),
            NodeStatus::Offline => Some(false),
            NodeStatus::Unknown => None,
        };
        ServerMetadataBuilder::default()
            .title(self.id.to_string())
            .icon(IconSpec::Named("building-symbolic".into()))
            .is_online(is_online)
            .build()
            .unwrap()
    }

    fn supported_adapters(&self) -> Vec<(Cow<str>, Cow<str>)> {
        if self.status == NodeStatus::Offline {
            vec![]
        } else {
            vec![
                (SpiceAdapter::TAG.into(), gettext("SPICE").into()),
                (VncAdapter::TAG.into(), gettext("VNC").into()),
                (VtePtyAdapter::TAG.into(), gettext("Console").into()),
            ]
        }
    }

    fn create_adapter(&self, tag: &str) -> LocalBoxFuture<ConnectionResult<Box<dyn Adapter>>> {
        create_proxmox_adapter(
            tag,
            &self.connection_id,
            self.id.as_ref(),
            self.client.clone(),
            ProxmoxEntity::Node(self.id.clone()),
        )
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            let client = self.client.clone();
            let connection_id = self.connection_id.clone();
            let node_id = self.id.clone();

            let map = run_on_tokio(async move {
                let mut server_map = ServerMapSend::default();

                for vm in client.node_lxc(&node_id).await.map_err(map_proxmox_error)? {
                    server_map.insert(
                        vm.vmid.to_string().into(),
                        Box::new(ProxmoxVm {
                            client: client.clone(),
                            connection_id: connection_id.clone(),
                            node_id: node_id.clone(),
                            vm_id: vm.vmid,
                            vm_type: VmType::Lxc,
                            name: vm.name,
                            status: vm.status,
                        }),
                    );
                }

                for vm in client
                    .node_qemu(&node_id, false)
                    .await
                    .map_err(map_proxmox_error)?
                {
                    server_map.insert(
                        vm.vmid.to_string().into(),
                        Box::new(ProxmoxVm {
                            client: client.clone(),
                            connection_id: connection_id.clone(),
                            node_id: node_id.clone(),
                            vm_id: vm.vmid,
                            vm_type: VmType::Qemu,
                            name: vm.name,
                            status: vm.status,
                        }),
                    );
                }

                Ok(server_map)
            })
            .await?;

            // TODO: Is this actually safe?
            let map_cast: ServerMap = unsafe { mem::transmute(map) };

            Ok(map_cast)
        })
    }
}

struct ProxmoxVm {
    client: Arc<ProxmoxApiClient>,
    connection_id: String,
    node_id: NodeId,
    vm_id: VmId,
    vm_type: VmType,
    name: Option<String>,
    status: VmStatus,
}

impl Actionable for ProxmoxVm {
    fn actions(&self) -> Vec<(Cow<'static, str>, Cow<'static, str>)> {
        if self.status == VmStatus::Running {
            match self.vm_type {
                VmType::Lxc => vec![
                    ("vmreboot".into(), gettext("Reboot").into()),
                    ("vmshutdown".into(), gettext("Shutdown").into()),
                    ("vmstop".into(), gettext("Force Poweroff").into()),
                ],
                VmType::Qemu => vec![
                    ("vmreboot".into(), gettext("Reboot").into()),
                    ("vmshutdown".into(), gettext("Shutdown").into()),
                    ("vmreset".into(), gettext("Force Reset").into()),
                    ("vmstop".into(), gettext("Force Poweroff").into()),
                ],
            }
        } else {
            vec![("vmstart".into(), gettext("Start / Resume").into())]
        }
    }

    fn action<'a>(&self, action_id: &str) -> Option<ServerAction<'a>> {
        match action_id {
            "vmreboot" => Some(self.act_reboot()),
            "vmshutdown" => Some(self.act_shutdown()),
            "vmreset" => Some(self.act_reset()),
            "vmstop" => Some(self.act_stop()),
            "vmstart" => Some(self.act_start()),
            _ => None,
        }
    }
}

impl ProxmoxVm {
    fn params(&self) -> ExecParams {
        ExecParams {
            client: self.client.clone(),
            node_id: Some(self.node_id.clone()),
            vm_id: Some(self.vm_id.clone()),
            vm_type: Some(self.vm_type),
        }
    }

    fn act_reboot<'a>(&self) -> ServerAction<'a> {
        ServerAction::new(
            Box::new(self.params()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let params = params.downcast::<ExecParams>().unwrap();

                    let (success, force_reload) = exec_cmd(
                        params,
                        |params| async move {
                            params
                                .client
                                .vm_reboot(
                                    &params.node_id.unwrap(),
                                    &params.vm_id.unwrap(),
                                    params.vm_type,
                                    Default::default(),
                                )
                                .await
                        },
                        || gettext("Reboot command successfully sent to VM."),
                        |err| {
                            warn!("failed VM reboot: {err:?}");
                            gettext("Failed to send reboot command.")
                        },
                        toov.as_ref(),
                    )
                    .await;
                    success || force_reload
                })
            }),
        )
    }

    fn act_shutdown<'a>(&self) -> ServerAction<'a> {
        ServerAction::new(
            Box::new(self.params()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let params = params.downcast::<ExecParams>().unwrap();

                    let (success, force_reload) = exec_cmd(
                        params,
                        |params| async move {
                            params
                                .client
                                .vm_shutdown(
                                    &params.node_id.unwrap(),
                                    &params.vm_id.unwrap(),
                                    params.vm_type,
                                    Default::default(),
                                )
                                .await
                        },
                        || gettext("Shutdown command successfully sent to VM."),
                        |err| {
                            warn!("failed VM shutdown: {err:?}");
                            gettext("Failed to send shutdown command.")
                        },
                        toov.as_ref(),
                    )
                    .await;
                    success || force_reload
                })
            }),
        )
    }

    fn act_reset<'a>(&self) -> ServerAction<'a> {
        ServerAction::new(
            Box::new(self.params()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let params = params.downcast::<ExecParams>().unwrap();

                    let (success, force_reload) = exec_cmd(
                        params,
                        |params| async move {
                            params
                                .client
                                .qemu_vm_reset(
                                    &params.node_id.unwrap(),
                                    &params.vm_id.unwrap(),
                                    Default::default(),
                                )
                                .await
                        },
                        || gettext("VM was successfully reset."),
                        |err| {
                            warn!("failed VM reset: {err:?}");
                            gettext("Failed to send reset command.")
                        },
                        toov.as_ref(),
                    )
                    .await;
                    success || force_reload
                })
            }),
        )
    }

    fn act_stop<'a>(&self) -> ServerAction<'a> {
        ServerAction::new(
            Box::new(self.params()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let params = params.downcast::<ExecParams>().unwrap();

                    let (success, force_reload) = exec_cmd(
                        params,
                        |params| async move {
                            params
                                .client
                                .vm_stop(
                                    &params.node_id.unwrap(),
                                    &params.vm_id.unwrap(),
                                    params.vm_type,
                                    Default::default(),
                                )
                                .await
                        },
                        || gettext("VM is now stopping."),
                        |err| {
                            warn!("failed stop: {err:?}");
                            gettext("Failed to send stop command.")
                        },
                        toov.as_ref(),
                    )
                    .await;
                    success || force_reload
                })
            }),
        )
    }

    fn act_start<'a>(&self) -> ServerAction<'a> {
        ServerAction::new(
            Box::new(self.params()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let params = params.downcast::<ExecParams>().unwrap();

                    let (success, force_reload) = exec_cmd(
                        params,
                        |params| async move {
                            params
                                .client
                                .vm_start(
                                    &params.node_id.unwrap(),
                                    &params.vm_id.unwrap(),
                                    params.vm_type,
                                    Default::default(),
                                )
                                .await
                        },
                        || gettext("VM is now starting."),
                        |err| {
                            warn!("failed stop: {err:?}");
                            gettext("Failed to send start command.")
                        },
                        toov.as_ref(),
                    )
                    .await;
                    success || force_reload
                })
            }),
        )
    }
}

impl ServerConnection for ProxmoxVm {
    fn metadata(&self) -> ServerMetadata {
        let is_online = match self.status {
            VmStatus::Running => Some(true),
            VmStatus::Stopped => Some(false),
            VmStatus::Unknown => None,
        };

        let title = match &self.name {
            None => self.vm_id.to_string(),
            Some(name) => format!("{} ({})", self.vm_id, name),
        };

        let icon = match self.vm_type {
            VmType::Lxc => IconSpec::Named("container-symbolic".into()),
            VmType::Qemu => IconSpec::Default,
        };

        ServerMetadataBuilder::default()
            .title(title)
            .icon(icon)
            .is_online(is_online)
            .build()
            .unwrap()
    }

    fn supported_adapters(&self) -> Vec<(Cow<str>, Cow<str>)> {
        macro_rules! SPICE {
            () => {
                (SpiceAdapter::TAG.into(), gettext("SPICE").into())
            };
        }
        macro_rules! VNC {
            () => {
                (VncAdapter::TAG.into(), gettext("VNC").into())
            };
        }
        macro_rules! TERM {
            () => {
                (VtePtyAdapter::TAG.into(), gettext("Console").into())
            };
        }

        if self.status != VmStatus::Running {
            vec![]
        } else {
            // TODO: Async?
            let res: proxmox_api::Result<Vec<_>> = tkruntime().block_on(async move {
                let mut adapters: Vec<(Cow<str>, Cow<str>)> = Vec::with_capacity(3);
                let supported = self
                    .client
                    .vm_available_console_proxies(&self.node_id, &self.vm_id, Some(self.vm_type))
                    .await?;

                if supported.as_ref().contains(&VmConsoleProxyType::Spice) {
                    adapters.push(SPICE!());
                }
                if supported.as_ref().contains(&VmConsoleProxyType::Vnc) {
                    adapters.push(VNC!());
                }
                if supported.as_ref().contains(&VmConsoleProxyType::Term) {
                    adapters.push(TERM!());
                }

                Ok(adapters)
            });

            res.unwrap_or_else(|err| {
                error!("Failed to load available connectors for a VM: {err:?}. Assume all.");
                vec![SPICE!(), VNC!(), TERM!()]
            })
        }
    }

    fn create_adapter(&self, tag: &str) -> LocalBoxFuture<ConnectionResult<Box<dyn Adapter>>> {
        create_proxmox_adapter(
            tag,
            &self.connection_id,
            &format!("{}/{}", self.node_id, self.vm_id),
            self.client.clone(),
            ProxmoxEntity::Vm(self.vm_type, self.node_id.clone(), self.vm_id.clone()),
        )
    }
}

fn map_proxmox_error(error: proxmox_api::Error) -> ConnectionError {
    match error {
        proxmox_api::Error::AuthFailed => ConnectionError::AuthFailed(None, error.into()),
        _ => ConnectionError::General(None, error.into()),
    }
}

struct ExecParams {
    client: Arc<ProxmoxApiClient>,
    node_id: Option<NodeId>,
    vm_id: Option<VmId>,
    vm_type: Option<VmType>,
}

async fn exec_cmd<F, Fut, S>(
    params: Box<ExecParams>,
    cmd: F,
    success_msg: impl (Fn() -> String) + Send + 'static,
    err_msg: impl (Fn(proxmox_api::Error) -> String) + Send + 'static,
    toov: Option<&adw::ToastOverlay>,
) -> (bool, bool)
where
    F: (Fn(Box<ExecParams>) -> Fut) + Send + 'static,
    Fut: Future<Output = Result<S, proxmox_api::Error>> + Send + 'static,
    S: Send,
{
    let (success, should_reload, text) = run_on_tokio(async move {
        let result = cmd(params).await;
        Ok((
            result.is_ok(),
            false,
            result.map(|_| success_msg()).unwrap_or_else(err_msg),
        ))
    })
    .await
    .unwrap_or_else(|e| {
        error!("Internal error running action: {e}");
        (
            false,
            true,
            gettext("Internal error while trying to execute command."),
        )
    });

    if success {
        // short sleep to maybe possibly give the chance to already have processed the task.
        // TODO: We could actually wait for the task to finish.
        sleep(Duration::from_millis(750)).await;
    }

    if let Some(toov) = toov {
        toov.add_toast(adw::Toast::builder().title(&text).timeout(5).build());
    }
    (success, should_reload)
}

enum ProxmoxEntity {
    Node(NodeId),
    Vm(VmType, NodeId, VmId),
}

enum AdapterCreds {
    Vnc(Vncproxy),
    Spice(Spiceproxy),
    Term(Termproxy),
}

fn create_proxmox_adapter<'a>(
    adapter_tag: &str,
    connection_id: &str,
    server_id: &str,
    client: Arc<ProxmoxApiClient>,
    entity: ProxmoxEntity,
) -> LocalBoxFuture<'a, ConnectionResult<Box<dyn Adapter>>> {
    let connection_id = connection_id.to_string();
    let server_id = server_id.to_string();
    let adapter_tag = adapter_tag.to_string();
    let adapter_type = match &*adapter_tag {
        SpiceAdapter::TAG => VmConsoleProxyType::Spice,
        VncAdapter::TAG => VmConsoleProxyType::Vnc,
        VtePtyAdapter::TAG => VmConsoleProxyType::Term,
        _ => {
            return Box::pin(async move {
                Err(ConnectionError::General(
                    None,
                    anyhow!("invalid adapter tag"),
                ))
            });
        }
    };

    Box::pin(run_on_tokio(async move {
        let adapter_creds = match &entity {
            ProxmoxEntity::Node(node_id) => match adapter_type {
                VmConsoleProxyType::Vnc => AdapterCreds::Vnc(
                    client
                        .node_vncshell(node_id, Default::default())
                        .await
                        .map_err(map_proxmox_error)?,
                ),
                VmConsoleProxyType::Spice => AdapterCreds::Spice(
                    client
                        .node_spiceshell(node_id, Default::default())
                        .await
                        .map_err(map_proxmox_error)?,
                ),
                VmConsoleProxyType::Term => AdapterCreds::Term(
                    client
                        .node_termproxy(node_id, Default::default())
                        .await
                        .map_err(map_proxmox_error)?,
                ),
            },
            ProxmoxEntity::Vm(vm_type, node_id, vm_id) => match adapter_type {
                VmConsoleProxyType::Vnc => AdapterCreds::Vnc(
                    client
                        .vm_vncproxy(node_id, vm_id, Some(*vm_type), Default::default())
                        .await
                        .map_err(map_proxmox_error)?,
                ),
                VmConsoleProxyType::Spice => AdapterCreds::Spice(
                    client
                        .vm_spiceproxy(node_id, vm_id, Some(*vm_type), Default::default())
                        .await
                        .map_err(map_proxmox_error)?,
                ),
                VmConsoleProxyType::Term => AdapterCreds::Term(
                    client
                        .vm_termproxy(node_id, vm_id, Some(*vm_type), Default::default())
                        .await
                        .map_err(map_proxmox_error)?
                        .1,
                ),
            },
        };

        let adapter: Box<dyn Adapter> = match adapter_creds {
            AdapterCreds::Vnc(vncproxy) => Box::new(VncAdapter::new_with_ca(
                client.clientconfig_hostname().to_string(),
                vncproxy.port.into(),
                vncproxy.user,
                vncproxy.ticket.into(),
                vncproxy.cert,
            )),
            AdapterCreds::Spice(spiceproxy) => Box::new(SpiceAdapter::new_with_custom_config(
                SpiceSessionConfigBuilder::default()
                    .host(Some(spiceproxy.host))
                    .password(Some(spiceproxy.password.into()))
                    .proxy(Some(spiceproxy.proxy))
                    .tls_port(Some(spiceproxy.tls_port))
                    .ca(spiceproxy
                        .ca
                        .map(|s| s.replace(r"\n", "\n"))
                        .map(String::into_bytes))
                    .cert_subject(spiceproxy.host_subject)
                    .build()
                    .unwrap(),
            )),
            AdapterCreds::Term(termproxy) => {
                let (node_id, vm_type, vm_id) = match entity {
                    ProxmoxEntity::Node(node_id) => {
                        (node_id.to_string(), String::new(), String::new())
                    }
                    ProxmoxEntity::Vm(vm_type, node_id, vm_id) => {
                        (vm_type.to_string(), node_id.to_string(), vm_id.to_string())
                    }
                };
                let ignore_ssl_errors = if client.clientconfig_ignore_ssl_errors() {
                    "1"
                } else {
                    "0"
                };

                Box::new(VtePtyAdapter::new(
                    connection_id,
                    server_id,
                    adapter_tag,
                    libexec_path(PTY_DRIVER_BIN).expect("failed to find libvirt vte driver in path. Is Field Monitor correctly installed?"),
                    vec![
                        client.clientconfig_connection_type().to_string(),
                        client.clientconfig_root().to_string(),
                        client.clientconfig_user_or_tokenid().to_string(),
                        client.clientconfig_password_or_apikey().unsecure().to_string(),
                        ignore_ssl_errors.to_string(),
                        node_id,
                        vm_type,
                        vm_id,
                        serde_json::to_string(&termproxy)
                            .map_err(|e| ConnectionError::General(
                                None, anyhow!("failed serialization: {e}").context(e)
                            ))?
                    ],
                ))
            }
        };

        Ok(adapter)
    }))
}
