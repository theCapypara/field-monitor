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
use serde::Serialize;
use std::num::NonZeroU32;

pub(crate) trait VmStatusInput {
    type LxcInput: Serialize;
    type QemuInput: Serialize;

    fn into_lxc(self) -> Self::LxcInput;
    fn into_qemu(self) -> Self::QemuInput;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VmStartInput {
    /// Ignore locks - only root is allowed to use this option.
    pub skiplock: Option<bool>,
    /// LXC only: If set, enables very verbose debug log-level on start.
    pub debug: Option<bool>,
    /// QEMU only: Override QEMU's -cpu argument with the given string.
    pub force_cpu: Option<String>, // force-cpu
    /// QEMU only: Specify the QEMU machine.
    pub machine: Option<String>,
    /// QEMU only: The cluster node name.
    pub migratedfrom: Option<String>,
    /// QEMU only: CIDR of the (sub) network that is used for migration.
    pub migration_network: Option<String>,
    /// QEMU only: Migration traffic is encrypted using an SSH tunnel by default. On secure, completely private networks this can be disabled to increase performance.
    pub migration_type: Option<String>, // todo enum: secure | insecure
    /// QEMU only: Some command save/restore state from this location.
    pub stateuri: Option<String>,
    /// QEMU only: Mapping from source to target storages. Providing only a single storage ID maps all source storages to that storage. Providing the special value '1' will map each source storage to itself.
    pub targetstorage: Option<String>,
    /// QEMU only: Wait maximal timeout seconds.
    pub timeout: Option<u64>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmStartInputLxc {
    /// Ignore locks - only root is allowed to use this option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skiplock: Option<u8>,
    /// If set, enables very verbose debug log-level on start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<u8>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmStartInputQemu {
    /// Ignore locks - only root is allowed to use this option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skiplock: Option<u8>,
    /// Override QEMU's -cpu argument with the given string.
    #[serde(rename = "force-cpu", skip_serializing_if = "Option::is_none")]
    pub force_cpu: Option<String>, // force-cpu
    /// Specify the QEMU machine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<String>,
    /// The cluster node name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migratedfrom: Option<String>,
    /// CIDR of the (sub) network that is used for migration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_network: Option<String>,
    /// Migration traffic is encrypted using an SSH tunnel by default. On secure, completely private networks this can be disabled to increase performance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_type: Option<String>, // todo enum: secure | insecure
    /// Some command save/restore state from this location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stateuri: Option<String>,
    /// Mapping from source to target storages. Providing only a single storage ID maps all source storages to that storage. Providing the special value '1' will map each source storage to itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targetstorage: Option<String>,
    /// Wait maximal timeout seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

impl VmStatusInput for VmStartInput {
    type LxcInput = VmStartInputLxc;
    type QemuInput = VmStartInputQemu;

    fn into_lxc(self) -> Self::LxcInput {
        VmStartInputLxc {
            skiplock: self.skiplock.map(|v| if v { 1 } else { 0 }),
            debug: self.debug.map(|v| if v { 1 } else { 0 }),
        }
    }

    fn into_qemu(self) -> Self::QemuInput {
        VmStartInputQemu {
            skiplock: self.skiplock.map(|v| if v { 1 } else { 0 }),
            force_cpu: self.force_cpu,
            machine: self.machine,
            migratedfrom: self.migratedfrom,
            migration_network: self.migration_network,
            migration_type: self.migration_type,
            stateuri: self.stateuri,
            targetstorage: self.targetstorage,
            timeout: self.timeout,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VmStopInput {
    /// Ignore locks - only root is allowed to use this option.
    pub skiplock: Option<bool>,
    /// Try to abort active 'vzshutdown' / 'qmshutdown' tasks before stopping.
    pub overrule_shutdown: Option<bool>, // overrule-shutdown
    /// QEMU only: Do not deactivate storage volumes.
    pub keep_active: Option<bool>, // keepActive
    /// QEMU only: The cluster node name.
    pub migratedfrom: Option<String>,
    /// QEMU only: Wait maximal timeout seconds.
    pub timeout: Option<u64>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmStopInputLxc {
    /// Ignore locks - only root is allowed to use this option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skiplock: Option<u8>,
    /// Try to abort active 'vzshutdown' tasks before stopping.
    #[serde(rename = "overrule-shutdown", skip_serializing_if = "Option::is_none")]
    pub overrule_shutdown: Option<u8>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmStopInputQemu {
    /// Ignore locks - only root is allowed to use this option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skiplock: Option<u8>,
    /// Try to abort active 'qmshutdown' tasks before stopping.
    #[serde(rename = "overrule-shutdown", skip_serializing_if = "Option::is_none")]
    pub overrule_shutdown: Option<u8>,
    /// Do not deactivate storage volumes.
    #[serde(rename = "keepActive", skip_serializing_if = "Option::is_none")]
    pub keep_active: Option<u8>, // keepActive
    /// The cluster node name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migratedfrom: Option<String>,
    /// Wait maximal timeout seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

impl VmStatusInput for VmStopInput {
    type LxcInput = VmStopInputLxc;
    type QemuInput = VmStopInputQemu;

    fn into_lxc(self) -> Self::LxcInput {
        VmStopInputLxc {
            skiplock: self.skiplock.map(|v| if v { 1 } else { 0 }),
            overrule_shutdown: self.overrule_shutdown.map(|v| if v { 1 } else { 0 }),
        }
    }

    fn into_qemu(self) -> Self::QemuInput {
        VmStopInputQemu {
            skiplock: self.skiplock.map(|v| if v { 1 } else { 0 }),
            overrule_shutdown: self.overrule_shutdown.map(|v| if v { 1 } else { 0 }),
            keep_active: self.keep_active.map(|v| if v { 1 } else { 0 }),
            migratedfrom: self.migratedfrom,
            timeout: self.timeout,
        }
    }
}

#[derive(Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct VmResetInputQemu {
    /// Ignore locks - only root is allowed to use this option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skiplock: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VmShutdownInput {
    /// Make sure the VM / Container stops.
    pub force_stop: Option<bool>, // forceStop
    /// Wait maximal timeout seconds.
    pub timeout: Option<u64>,
    /// QEMU only: Do not deactivate storage volumes.
    pub keep_active: Option<bool>, // keepActive
    /// QEMU only: Ignore locks - only root is allowed to use this option.
    pub skiplock: Option<bool>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmShutdownInputLxc {
    /// Make sure the VM / Container stops.
    #[serde(rename = "forceStop", skip_serializing_if = "Option::is_none")]
    pub force_stop: Option<u8>,
    /// Wait maximal timeout seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmShutdownInputQemu {
    /// Make sure the VM / Container stops.
    #[serde(rename = "forceStop", skip_serializing_if = "Option::is_none")]
    pub force_stop: Option<u8>,
    /// Wait maximal timeout seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// QEMU only: Do not deactivate storage volumes.
    #[serde(rename = "keepActive", skip_serializing_if = "Option::is_none")]
    pub keep_active: Option<u8>,
    /// QEMU only: Ignore locks - only root is allowed to use this option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skiplock: Option<u8>,
}

impl VmStatusInput for VmShutdownInput {
    type LxcInput = VmShutdownInputLxc;
    type QemuInput = VmShutdownInputQemu;

    fn into_lxc(self) -> Self::LxcInput {
        VmShutdownInputLxc {
            force_stop: self.force_stop.map(|v| if v { 1 } else { 0 }),
            timeout: self.timeout,
        }
    }

    fn into_qemu(self) -> Self::QemuInput {
        VmShutdownInputQemu {
            force_stop: self.force_stop.map(|v| if v { 1 } else { 0 }),
            timeout: self.timeout,
            keep_active: self.keep_active.map(|v| if v { 1 } else { 0 }),
            skiplock: self.skiplock.map(|v| if v { 1 } else { 0 }),
        }
    }
}

#[derive(Serialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct VmRebootInput {
    /// Wait maximal timeout seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

impl VmStatusInput for VmRebootInput {
    type LxcInput = VmRebootInput;
    type QemuInput = VmRebootInput;

    fn into_lxc(self) -> Self::LxcInput {
        self
    }

    fn into_qemu(self) -> Self::QemuInput {
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VmSuspendInput {
    /// QEMU only: Ignore locks - only root is allowed to use this option.
    pub skiplock: Option<bool>,
    /// QEMU only: The storage for the VM state
    pub statestorage: Option<String>,
    /// QEMU only: If set, suspends the VM to disk. Will be resumed on next VM start.
    pub todisk: Option<bool>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmSuspendInputLxc {}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmSuspendInputQemu {
    /// Ignore locks - only root is allowed to use this option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skiplock: Option<u8>,
    /// The storage for the VM state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statestorage: Option<String>,
    /// If set, suspends the VM to disk. Will be resumed on next VM start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todisk: Option<u8>,
}

impl VmStatusInput for VmSuspendInput {
    type LxcInput = VmSuspendInputLxc;
    type QemuInput = VmSuspendInputQemu;

    fn into_lxc(self) -> Self::LxcInput {
        VmSuspendInputLxc {}
    }

    fn into_qemu(self) -> Self::QemuInput {
        VmSuspendInputQemu {
            skiplock: self.skiplock.map(|v| if v { 1 } else { 0 }),
            statestorage: self.statestorage,
            todisk: self.todisk.map(|v| if v { 1 } else { 0 }),
        }
    }
}

#[derive(Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct NodeTermproxyInput {
    /// Run specific command or default to login (requires 'root@pam')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<NodeTermproxyCmd>,
    /// Add parameters to a command. Encoded as null terminated strings.
    #[serde(rename = "cmd-opts", skip_serializing_if = "Option::is_none")]
    pub cmd_opts: Option<String>,
}

#[derive(Eq, PartialEq, Serialize, Debug, Clone, Copy)]
pub enum NodeTermproxyCmd {
    #[serde(rename = "login")]
    Login,
    #[serde(rename = "ceph_install")]
    CephInstall,
    #[serde(rename = "upgrade")]
    Upgrade,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VmTermproxyInput {
    /// LXC only: opens a serial terminal (defaults to display)
    pub serial: Option<VmTermproxySerial>,
}

#[derive(Eq, PartialEq, Serialize, Debug, Clone, Copy)]
pub enum VmTermproxySerial {
    #[serde(rename = "serial0")]
    Serial0,
    #[serde(rename = "serial1")]
    Serial1,
    #[serde(rename = "serial2")]
    Serial2,
    #[serde(rename = "serial3")]
    Serial3,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmTermproxyInputLxc {
    /// opens a serial terminal (defaults to display)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<VmTermproxySerial>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmTermproxyInputQemu {}

impl VmStatusInput for VmTermproxyInput {
    type LxcInput = VmTermproxyInputLxc;
    type QemuInput = VmTermproxyInputQemu;

    fn into_lxc(self) -> Self::LxcInput {
        VmTermproxyInputLxc {
            serial: self.serial,
        }
    }

    fn into_qemu(self) -> Self::QemuInput {
        VmTermproxyInputQemu {}
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct VncwebsocketInput {
    /// Port number returned by previous vncproxy call.
    pub port: NonZeroU32,
    /// Ticket from previous call to vncproxy.
    pub vncticket: String,
}

#[derive(Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct NodeSpiceshellInput {
    /// Run specific command or default to login (requires 'root@pam')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<NodeTermproxyCmd>,
    /// Add parameters to a command. Encoded as null terminated strings.
    #[serde(rename = "cmd-opts", skip_serializing_if = "Option::is_none")]
    pub cmd_opts: Option<String>,
    /// SPICE proxy server. This can be used by the client to specify the proxy server.
    /// All nodes in a cluster runs 'spiceproxy', so it is up to the client to choose one.
    /// By default, we return the node where the VM is currently running. As reasonable setting
    /// is to use same node you use to connect to the API (This is window.location.hostname for
    /// the JS GUI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
}

#[derive(Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct VmSpiceproxyInput {
    /// SPICE proxy server. This can be used by the client to specify the proxy server.
    /// All nodes in a cluster runs 'spiceproxy', so it is up to the client to choose one.
    /// By default, we return the node where the VM is currently running. As reasonable setting
    /// is to use same node you use to connect to the API (This is window.location.hostname for
    /// the JS GUI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
}

#[derive(Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct NodeVncshellInput {
    /// Run specific command or default to login (requires 'root@pam')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<NodeTermproxyCmd>,
    /// Add parameters to a command. Encoded as null terminated strings.
    #[serde(rename = "cmd-opts", skip_serializing_if = "Option::is_none")]
    pub cmd_opts: Option<String>,
    /// sets the height of the console in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u64>,
    /// use websocket instead of standard vnc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub websocket: Option<u8>,
    /// sets the width of the console in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VmVncproxyInput {
    /// LXC only: sets the height of the console in pixels.
    pub height: Option<u64>,
    /// use websocket instead of standard vnc.
    pub websocket: Option<u8>,
    /// LXC only: sets the width of the console in pixels.
    pub width: Option<u64>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmVncproxyInputLxc {
    /// sets the height of the console in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u64>,
    /// use websocket instead of standard vnc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub websocket: Option<u8>,
    /// sets the width of the console in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u64>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmVncproxyInputQemu {
    /// use websocket instead of standard vnc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub websocket: Option<u8>,
}

impl VmStatusInput for VmVncproxyInput {
    type LxcInput = VmVncproxyInputLxc;
    type QemuInput = VmVncproxyInputQemu;

    fn into_lxc(self) -> Self::LxcInput {
        VmVncproxyInputLxc {
            height: self.height,
            websocket: self.websocket,
            width: self.width,
        }
    }

    fn into_qemu(self) -> Self::QemuInput {
        VmVncproxyInputQemu {
            websocket: self.websocket,
        }
    }
}
