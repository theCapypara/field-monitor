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
use crate::api;
use crate::api::cache::InfoFetcher;
use crate::api::{ExecParams, ProxmoxEntity};
use crate::tokiort::{run_on_tokio, tkruntime};
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use libfieldmonitor::adapter::spice::SpiceAdapter;
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::adapter::vte_pty::VtePtyAdapter;
use libfieldmonitor::connection::{
    Actionable, ConnectionResult, IconSpec, ServerAction, ServerConnection, ServerMetadata,
    ServerMetadataBuilder,
};
use log::{error, warn};
use proxmox_api::{NodeId, VmConsoleProxyType, VmId, VmStatus, VmType};
use std::borrow::Cow;
use std::sync::Arc;

pub struct ProxmoxVm {
    pub info_fetcher: Arc<InfoFetcher>,
    pub connection_id: String,
    pub node_id: NodeId,
    pub vm_id: VmId,
    pub vm_type: VmType,
    pub name: Option<String>,
}

impl ProxmoxVm {
    async fn status(&self) -> VmStatus {
        let info_fetcher = self.info_fetcher.clone();
        let node_id = self.node_id.clone();
        let vm_type = self.vm_type;
        let vm_id = self.vm_id.clone();
        run_on_tokio(async move { Ok(info_fetcher.vm_status(&node_id, vm_type, &vm_id).await) })
            .await
            .unwrap()
    }
}

impl Actionable for ProxmoxVm {
    fn actions(&self) -> LocalBoxFuture<Vec<(Cow<'static, str>, Cow<'static, str>)>> {
        Box::pin(async move {
            if self.status().await == VmStatus::Running {
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
        })
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
            client: self.info_fetcher.client.clone(),
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

                    api::exec_cmd(
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

                    api::exec_cmd(
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

                    api::exec_cmd(
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

                    api::exec_cmd(
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

                    api::exec_cmd(
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
                })
            }),
        )
    }
}

impl ServerConnection for ProxmoxVm {
    fn metadata(&self) -> LocalBoxFuture<ServerMetadata> {
        Box::pin(async move {
            let is_online = match self.status().await {
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
        })
    }

    fn supported_adapters(&self) -> LocalBoxFuture<Vec<(Cow<str>, Cow<str>)>> {
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

        Box::pin(async move {
            if self.status().await != VmStatus::Running {
                vec![]
            } else {
                // TODO: Async?
                let res: proxmox_api::Result<Vec<_>> = tkruntime().block_on(async move {
                    let mut adapters: Vec<(Cow<str>, Cow<str>)> = Vec::with_capacity(3);
                    let supported = self
                        .info_fetcher
                        .client
                        .vm_available_console_proxies(
                            &self.node_id,
                            &self.vm_id,
                            Some(self.vm_type),
                        )
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
        })
    }

    fn create_adapter(&self, tag: &str) -> LocalBoxFuture<ConnectionResult<Box<dyn Adapter>>> {
        api::create_proxmox_adapter(
            tag,
            &self.connection_id,
            &format!("{}/{}", self.node_id, self.vm_id),
            self.info_fetcher.client.clone(),
            ProxmoxEntity::Vm(self.vm_type, self.node_id.clone(), self.vm_id.clone()),
        )
    }
}
