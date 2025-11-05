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
use crate::api;
use crate::api::cache::InfoFetcher;
use crate::api::vm::ProxmoxVm;
use crate::api::{ExecParams, ProxmoxEntity};
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use libfieldmonitor::adapter::spice::SpiceAdapter;
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::adapter::vte_pty::VtePtyAdapter;
use libfieldmonitor::connection::{
    Actionable, ConnectionError, ConnectionResult, IconSpec, ServerAction, ServerConnection,
    ServerMap, ServerMapSend, ServerMetadata, ServerMetadataBuilder,
};
use libfieldmonitor::tokiort::run_on_tokio;
use log::warn;
use proxmox_api::{NodeId, NodeStatus, VmType};
use std::borrow::Cow;
use std::mem::transmute;
use std::sync::Arc;

pub struct ProxmoxNode {
    pub info_fetcher: Arc<InfoFetcher>,
    pub connection_id: String,
    pub id: NodeId,
}

impl ProxmoxNode {
    async fn status(&self) -> NodeStatus {
        let info_fetcher = self.info_fetcher.clone();
        let id = self.id.clone();
        run_on_tokio::<_, _, anyhow::Error>(async move { Ok(info_fetcher.node_status(&id).await) })
            .await
            .unwrap()
    }
}

impl Actionable for ProxmoxNode {
    fn actions(&self) -> LocalBoxFuture<'_, Vec<(Cow<'static, str>, Cow<'static, str>)>> {
        Box::pin(async move {
            if self.status().await != NodeStatus::Offline {
                vec![
                    ("nodereboot".into(), gettext("Reboot").into()),
                    ("nodeshutdown".into(), gettext("Shutdown").into()),
                ]
            } else {
                vec![]
            }
        })
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
            client: self.info_fetcher.client.clone(),
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

                    api::exec_cmd(
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
                })
            }),
        )
    }
}

impl ServerConnection for ProxmoxNode {
    fn metadata(&self) -> LocalBoxFuture<'_, ServerMetadata> {
        Box::pin(async move {
            let is_online = match self.status().await {
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
        })
    }

    fn supported_adapters(&self) -> LocalBoxFuture<'_, Vec<(Cow<'_, str>, Cow<'_, str>)>> {
        Box::pin(async move {
            if self.status().await == NodeStatus::Offline {
                vec![]
            } else {
                vec![
                    (SpiceAdapter::TAG.into(), gettext("SPICE").into()),
                    (VncAdapter::TAG.into(), gettext("VNC").into()),
                    (VtePtyAdapter::TAG.into(), gettext("Console").into()),
                ]
            }
        })
    }

    fn create_adapter(&self, tag: &str) -> LocalBoxFuture<'_, ConnectionResult<Box<dyn Adapter>>> {
        api::create_proxmox_adapter(
            tag,
            &self.connection_id,
            self.id.as_ref(),
            self.info_fetcher.client.clone(),
            ProxmoxEntity::Node(self.id.clone()),
        )
    }

    fn servers(&self) -> LocalBoxFuture<'_, ConnectionResult<ServerMap>> {
        Box::pin(async move {
            let info_fetcher = self.info_fetcher.clone();
            let connection_id = self.connection_id.clone();
            let node_id = self.id.clone();

            let map = run_on_tokio::<_, _, ConnectionError>(async move {
                let mut server_map = ServerMapSend::default();

                for vm in info_fetcher.lxcs(&node_id).await? {
                    server_map.insert(
                        vm.vmid.to_string().into(),
                        Box::new(ProxmoxVm {
                            info_fetcher: info_fetcher.clone(),
                            connection_id: connection_id.clone(),
                            node_id: node_id.clone(),
                            vm_id: vm.vmid.clone(),
                            vm_type: VmType::Lxc,
                            name: vm.name,
                        }),
                    );
                }

                for vm in info_fetcher.qemus(&node_id).await? {
                    server_map.insert(
                        vm.vmid.to_string().into(),
                        Box::new(ProxmoxVm {
                            info_fetcher: info_fetcher.clone(),
                            connection_id: connection_id.clone(),
                            node_id: node_id.clone(),
                            vm_id: vm.vmid.clone(),
                            vm_type: VmType::Qemu,
                            name: vm.name,
                        }),
                    );
                }

                Ok(server_map)
            })
            .await?;

            // TODO: Is this actually safe?
            let map_cast: ServerMap = unsafe { transmute(map) };

            Ok(map_cast)
        })
    }
}
