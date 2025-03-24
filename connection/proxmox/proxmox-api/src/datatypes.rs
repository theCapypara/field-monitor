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
use std::fmt::{Display, Formatter};
use std::num::NonZeroU32;

use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;

pub use crate::datatypes::ids::*;
pub use crate::datatypes::params::*;

mod ids;
mod params;

/// Single element of response of GET /nodes
///
/// https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes
#[derive(PartialEq, Deserialize, Debug, Clone)]
pub struct Node {
    /// The cluster node name.
    pub node: NodeId,
    /// The cluster node name.
    pub status: NodeStatus,
    /// CPU utilization.
    #[serde(default)]
    pub cpu: Option<f64>,
    /// Support level.
    #[serde(default)]
    pub level: Option<String>,
    /// Number of available CPUs.
    #[serde(default)]
    pub maxcpu: Option<i64>,
    /// Number of available memory in bytes.
    #[serde(default)]
    pub maxmem: Option<i64>,
    /// Used memory in bytes.
    #[serde(default)]
    pub mem: Option<i64>,
    /// The SSL fingerprint for the node certificate.
    #[serde(default)]
    pub ssl_fingerprint: Option<String>,
    /// Node uptime in seconds.
    #[serde(default)]
    pub uptime: Option<i64>,
}

/// Status of a node
#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Copy)]
pub enum NodeStatus {
    #[serde(rename = "online")]
    Online,
    #[serde(rename = "offline")]
    Offline,
    #[serde(rename = "unknown", other)]
    Unknown,
}

/// Single element of response of GET /node/{node}/lxc
///
/// https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/lxc
#[derive(PartialEq, Deserialize, Debug, Clone)]
pub struct LxcVm {
    /// LXC Container status.
    pub status: VmStatus,
    /// The (unique) ID of the VM.
    pub vmid: VmId,
    /// Maximum usable CPUs.
    #[serde(default)]
    pub cpus: Option<f64>,
    /// The current config lock, if any.
    #[serde(default)]
    pub lock: Option<String>,
    /// Root disk size in bytes.
    #[serde(default)]
    pub maxdisk: Option<i64>,
    /// Maximum memory in bytes.
    #[serde(default)]
    pub maxmem: Option<i64>,
    /// Maximum SWAP memory in bytes.
    #[serde(default)]
    pub maxswap: Option<i64>,
    /// Container name.
    #[serde(default)]
    pub name: Option<String>,
    /// The current configured tags, if any.
    #[serde(default)]
    pub tags: Option<String>,
    /// Uptime.
    #[serde(default)]
    pub uptime: Option<i64>,
}

/// Single element of response of GET /node/{node}/qemu
///
/// https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu
#[derive(PartialEq, Deserialize, Debug, Clone)]
pub struct QemuVm {
    /// QEMU process status.
    pub status: VmStatus,
    /// The (unique) ID of the VM.
    pub vmid: VmId,
    /// Maximum usable CPUs.
    #[serde(default)]
    pub cpus: Option<f64>,
    /// The current config lock, if any.
    #[serde(default)]
    pub lock: Option<String>,
    /// Root disk size in bytes.
    #[serde(default)]
    pub maxdisk: Option<i64>,
    /// Maximum memory in bytes.
    #[serde(default)]
    pub maxmem: Option<i64>,
    /// VM name.
    #[serde(default)]
    pub name: Option<String>,
    /// PID of running qemu process.
    #[serde(default)]
    pub pid: Option<i64>,
    /// VM run state from the 'query-status' QMP monitor command.
    #[serde(default)]
    pub qmpstatus: Option<String>,
    /// The currently running machine type (if running).
    #[serde(default, rename = "running-machine")]
    pub running_machine: Option<String>,
    /// The currently running QEMU version (if running).
    #[serde(default, rename = "running-qemu")]
    pub running_qemu: Option<String>,
    /// The current configured tags, if any.
    #[serde(default)]
    pub tags: Option<String>,
    /// Uptime.
    #[serde(default)]
    pub uptime: Option<i64>,
}

/// Single element of response of GET /node/{node}/qemu/{vmid}/status/current
///
/// https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid}/status/current
#[derive(PartialEq, Deserialize, Debug, Clone)]
pub struct QemuVmStatus {
    /// HA manager service status.
    pub ha: Value,
    /// QEMU process status.
    pub status: VmStatus,
    /// The (unique) ID of the VM.
    pub vmid: VmId,
    /// QEMU Guest Agent is enabled in config.
    #[serde(default, deserialize_with = "deserialize_opt_int_bool")]
    pub agent: Option<bool>,
    /// Enable a specific clipboard. If not set, depending on the display type the SPICE one will be added.
    #[serde(default)]
    pub clipboard: Option<Value>,
    /// Maximum usable CPUs.
    #[serde(default)]
    pub cpus: Option<f64>,
    /// The current config lock, if any.
    #[serde(default)]
    pub lock: Option<String>,
    /// Root disk size in bytes.
    #[serde(default)]
    pub maxdisk: Option<i64>,
    /// Maximum memory in bytes.
    #[serde(default)]
    pub maxmem: Option<i64>,
    /// VM name.
    #[serde(default)]
    pub name: Option<String>,
    /// PID of running qemu process.
    #[serde(default)]
    pub pid: Option<i64>,
    /// VM run state from the 'query-status' QMP monitor command.
    #[serde(default)]
    pub qmpstatus: Option<String>,
    /// The currently running machine type (if running).
    #[serde(default, rename = "running-machine")]
    pub running_machine: Option<String>,
    /// The currently running QEMU version (if running).
    #[serde(default, rename = "running-qemu")]
    pub running_qemu: Option<String>,
    /// QEMU VGA configuration supports spice.
    #[serde(default, deserialize_with = "deserialize_opt_int_bool")]
    pub spice: Option<bool>,
    /// The current configured tags, if any.
    #[serde(default)]
    pub tags: Option<String>,
    /// Uptime.
    #[serde(default)]
    pub uptime: Option<i64>,
}

/// Status of a VM
#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Copy)]
pub enum VmStatus {
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "unknown", other)]
    Unknown,
}

/// Return value of termproxy API endpoints:
///
/// - https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/termproxy
/// - https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/lxc/{vmid}/termproxy
/// - https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid}/termproxy
#[derive(PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct Termproxy {
    #[serde(deserialize_with = "try_deserialize_port_from_str")]
    pub port: NonZeroU32,
    pub ticket: String,
    pub upid: String,
    pub user: String,
}

/// Return value of spiceproxy/spiceshell API endpoints:
///
/// - https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/spiceshell
/// - https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/lxc/{vmid}/spiceproxy
/// - https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid}/spiceproxy
#[derive(PartialEq, Deserialize, Debug, Clone)]
pub struct Spiceproxy {
    pub host: String,
    pub password: String,
    pub proxy: String,
    #[serde(
        rename = "tls-port",
        deserialize_with = "try_deserialize_port_from_str"
    )]
    pub tls_port: NonZeroU32,
    pub r#type: String,
    #[serde(default)]
    pub ca: Option<String>,
    #[serde(rename = "host-subject", default)]
    pub host_subject: Option<String>,
}

/// Return value of vncproxy/vncshell API endpoints:
///
/// - https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/vncshell
/// - https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/lxc/{vmid}/vncproxy
/// - https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid}/vncproxy
#[derive(PartialEq, Deserialize, Debug, Clone)]
pub struct Vncproxy {
    pub cert: String,
    #[serde(deserialize_with = "try_deserialize_port_from_str")]
    pub port: NonZeroU32,
    pub ticket: String,
    pub upid: String,
    pub user: String,
}

#[derive(Eq, PartialEq, Deserialize, Debug, Clone)]
pub(crate) struct Ticket {
    pub ticket: String,
    #[serde(rename = "CSRFPreventionToken")]
    pub csrf_prevention_token: String,
}

#[derive(Eq, PartialEq, Deserialize, Debug, Clone)]
pub(crate) struct Wrapper<T> {
    pub data: Option<T>,
    pub reason: Option<String>,
}

/// Type of a VM.
#[derive(Clone, Copy, Debug)]
pub enum VmType {
    Lxc,
    Qemu,
}

impl Display for VmType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VmType::Lxc => "lxc".fmt(f),
            VmType::Qemu => "qemu".fmt(f),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmConsoleProxyType {
    Term,
    Spice,
    Vnc,
}

fn deserialize_opt_int_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: Option<u8> = de::Deserialize::deserialize(deserializer)?;

    Ok(s.map(|v| v > 0))
}

fn try_deserialize_port_from_str<'de, D>(deserializer: D) -> Result<NonZeroU32, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MaybeNonZeroU32<'a> {
        String(&'a str),
        NonZeroU32(NonZeroU32),
    }

    match MaybeNonZeroU32::deserialize(deserializer)? {
        MaybeNonZeroU32::String(string) => {
            if let Ok(num) = string.parse::<u32>() {
                if let Some(non_zero) = NonZeroU32::new(num) {
                    return Ok(non_zero);
                }
            }
            Err(Error::invalid_type(
                Unexpected::Str(string),
                &"a non-zero integer",
            ))
        }
        MaybeNonZeroU32::NonZeroU32(v) => Ok(v),
    }
}
