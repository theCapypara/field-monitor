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
use libfieldmonitor::cache::{Cached, LoadCacheObject};
use libfieldmonitor::connection::ConnectionResult;
use proxmox_api::{
    LxcVm, Node, NodeId, NodeStatus, ProxmoxApiClient, QemuVm, VmId, VmStatus, VmType,
};
use std::collections::HashMap;
use std::sync::Arc;

pub struct InfoFetcher {
    pub client: Arc<ProxmoxApiClient>,
    nodes: Cached<NodeCache>,
    lxc_vms: tokio::sync::Mutex<HashMap<NodeId, Cached<LxcVmsCache>>>,
    qemu_vms: tokio::sync::Mutex<HashMap<NodeId, Cached<QemuVmsCache>>>,
}

#[allow(clippy::useless_asref)] // This is a false positive
impl InfoFetcher {
    pub fn new(client: Arc<ProxmoxApiClient>) -> Self {
        Self {
            nodes: Cached::new(client.clone()),
            lxc_vms: Default::default(),
            qemu_vms: Default::default(),
            client,
        }
    }

    pub async fn nodes(&self) -> ConnectionResult<Vec<Node>> {
        self.nodes
            .get()
            .await
            .0
            .as_ref()
            .map(Clone::clone)
            .map_err(api::map_proxmox_error_ref)
    }

    pub async fn lxcs(&self, node_id: &NodeId) -> ConnectionResult<Vec<LxcVm>> {
        let mut lock = self.lxc_vms.lock().await;
        let cache = lock
            .entry(node_id.clone())
            .or_insert_with(|| Cached::new((self.client.clone(), node_id.clone())));

        cache
            .get()
            .await
            .0
            .as_ref()
            .map(Clone::clone)
            .map_err(api::map_proxmox_error_ref)
    }

    pub async fn qemus(&self, node_id: &NodeId) -> ConnectionResult<Vec<QemuVm>> {
        let mut lock = self.qemu_vms.lock().await;
        let cache = lock
            .entry(node_id.clone())
            .or_insert_with(|| Cached::new((self.client.clone(), node_id.clone())));

        cache
            .get()
            .await
            .0
            .as_ref()
            .map(Clone::clone)
            .map_err(api::map_proxmox_error_ref)
    }

    pub async fn node_status(&self, node_id: &NodeId) -> NodeStatus {
        let Ok(nodes) = &self.nodes.get().await.0 else {
            return NodeStatus::Unknown;
        };
        for node in nodes {
            if &node.node == node_id {
                return node.status;
            }
        }
        NodeStatus::Unknown
    }

    pub async fn vm_status(&self, node_id: &NodeId, vm_type: VmType, vm_id: &VmId) -> VmStatus {
        match vm_type {
            VmType::Lxc => {
                let Ok(vms) = self.lxcs(node_id).await else {
                    return VmStatus::Unknown;
                };
                for vm in vms {
                    if &vm.vmid == vm_id {
                        return vm.status;
                    }
                }
            }
            VmType::Qemu => {
                let Ok(vms) = self.qemus(node_id).await else {
                    return VmStatus::Unknown;
                };
                for vm in vms {
                    if &vm.vmid == vm_id {
                        return vm.status;
                    }
                }
            }
        }
        VmStatus::Unknown
    }
}

struct NodeCache(proxmox_api::Result<Vec<Node>>);

impl LoadCacheObject for NodeCache {
    type Params = Arc<ProxmoxApiClient>;

    async fn construct(_previous_value: Option<Self>, params: &Self::Params) -> Self
    where
        Self: Sized,
    {
        Self(params.nodes().await)
    }
}

struct LxcVmsCache(proxmox_api::Result<Vec<LxcVm>>);

impl LoadCacheObject for LxcVmsCache {
    type Params = (Arc<ProxmoxApiClient>, NodeId);

    async fn construct(_previous_value: Option<Self>, params: &Self::Params) -> Self
    where
        Self: Sized,
    {
        let (client, node) = params;
        Self(client.node_lxc(node).await)
    }
}

struct QemuVmsCache(proxmox_api::Result<Vec<QemuVm>>);

impl LoadCacheObject for QemuVmsCache {
    type Params = (Arc<ProxmoxApiClient>, NodeId);

    async fn construct(_previous_value: Option<Self>, params: &Self::Params) -> Self
    where
        Self: Sized,
    {
        let (client, node) = params;
        Self(client.node_qemu(node, false).await)
    }
}
