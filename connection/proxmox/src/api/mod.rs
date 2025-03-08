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
use crate::tokiort::run_on_tokio;
use anyhow::anyhow;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use libfieldmonitor::adapter::spice::{SpiceAdapter, SpiceSessionConfigBuilder};
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::adapter::vte_pty::VtePtyAdapter;
use libfieldmonitor::connection::{ConnectionError, ConnectionResult};
use libfieldmonitor::libexec_path;
use log::error;
use proxmox_api::{
    NodeId, ProxmoxApiClient, Spiceproxy, Termproxy, VmConsoleProxyType, VmId, VmType, Vncproxy,
};
use std::future::Future;
use std::sync::Arc;

mod cache;
mod connection;
mod node;
mod provider;
mod vm;

pub use provider::{ProxmoxConnectionProvider, ProxmoxConnectionProviderConstructor};

pub const PTY_DRIVER_BIN: &str = "de.capypara.FieldMonitor.PtyDrv.Proxmox";

fn map_proxmox_error(error: proxmox_api::Error) -> ConnectionError {
    match error {
        proxmox_api::Error::AuthFailed => ConnectionError::AuthFailed(None, error.into()),
        _ => ConnectionError::General(None, error.into()),
    }
}

fn map_proxmox_error_ref(error: &proxmox_api::Error) -> ConnectionError {
    match error {
        proxmox_api::Error::AuthFailed => ConnectionError::AuthFailed(None, anyhow!("{}", error)),
        _ => ConnectionError::General(None, anyhow!("{}", error)),
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
) where
    F: (Fn(Box<ExecParams>) -> Fut) + Send + 'static,
    Fut: Future<Output = Result<S, proxmox_api::Error>> + Send + 'static,
    S: Send,
{
    let text = run_on_tokio(async move {
        let result = cmd(params).await;
        Ok(result.map(|_| success_msg()).unwrap_or_else(err_msg))
    })
    .await
    .unwrap_or_else(|e| {
        error!("Internal error running action: {e}");
        gettext("Internal error while trying to execute command.")
    });

    if let Some(toov) = toov {
        toov.add_toast(adw::Toast::builder().title(&text).timeout(5).build());
    }
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
