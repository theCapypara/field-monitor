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
use std::process::exit;
use std::str::FromStr;
use std::time::Duration;

use clap::{Parser, Subcommand};
use log::{debug, info};
use secure_string::SecureString;
use simple_logger::SimpleLogger;
use tokio::time::sleep;

use proxmox_api::{NodeId, ProxmoxApiClient, VmId};

/// Minimal API CLI client for Proxmox
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct MainCliArgs {
    /// Proxmox API root URL
    #[arg(short, long, global = true, default_value = "")] // global args can't be required
    url: String,
    /// Username. If set also requires password and no apikey/tokenid.
    #[arg(long, env = "PROXMOX_USER")]
    username: Option<String>,
    /// Password. If set also requires user and no apikey/tokenid.
    #[arg(long, env = "PROXMOX_PASS")]
    password: Option<String>,
    /// Token ID. If set also requires apikey and no username/password.
    #[arg(long, env = "PROXMOX_APIKEY")]
    tokenid: Option<String>,
    /// API Key. If set also requires tokenid and no username/password.
    #[arg(long, env = "PROXMOX_TOKENID")]
    apikey: Option<String>,
    /// Ignore SSL verification
    #[arg(short = 'k', long, global = true)]
    ignore_ssl_errors: bool,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List nodes
    Nodes,
    /// List nodes forever, waiting between attempts
    NodesForever,
    /// List container VMs
    ListVmsLxc { node: String },
    /// List QEMU VMs
    ListVmsQemu { node: String },
    /// Reboot a node
    RebootNode { node: String },
    /// Shutdown a node
    ShutdownNode { node: String },
    /// List supported access adapters for a VM.
    Adapters {
        node: String,
        vmid: u64,
        #[arg(value_enum, long, default_value_t)]
        vm_type: VmType,
    },
    /// Start a VM
    Start {
        node: String,
        vmid: u64,
        #[arg(value_enum, long, default_value_t)]
        vm_type: VmType,
    },
    /// Stop a VM
    Stop {
        node: String,
        vmid: u64,
        #[arg(value_enum, long, default_value_t)]
        vm_type: VmType,
    },
    /// Restart a VM
    Reboot {
        node: String,
        vmid: u64,
        #[arg(value_enum, long, default_value_t)]
        vm_type: VmType,
    },
    /// Shutdown a VM
    Shutdown {
        node: String,
        vmid: u64,
        #[arg(value_enum, long, default_value_t)]
        vm_type: VmType,
    },
    /// Reset a QEMU VM
    Reset { node: String, vmid: u64 },
    /// Suspend a VM
    Suspend {
        node: String,
        vmid: u64,
        #[arg(value_enum, long, default_value_t)]
        vm_type: VmType,
    },
    /// Get SPICE connection data for node
    SpiceNode { node: String },
    /// Get VNC connection data for node
    VncNode { node: String },
    /// Get terminal proxy connection data for node
    TermNode { node: String },
    /// Get SPICE connection data for VM
    Spice {
        node: String,
        vmid: u64,
        #[arg(value_enum, long, default_value_t)]
        vm_type: VmType,
    },
    /// Get VNC connection data for VM
    Vnc {
        node: String,
        vmid: u64,
        #[arg(value_enum, long, default_value_t)]
        vm_type: VmType,
    },
    /// Get terminal proxy connection data for VM
    Term {
        node: String,
        vmid: u64,
        #[arg(value_enum, long, default_value_t)]
        vm_type: VmType,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, Default, Debug)]
enum VmType {
    #[default]
    Auto,
    Lxc,
    Qemu,
}

impl From<VmType> for Option<proxmox_api::VmType> {
    fn from(value: VmType) -> Self {
        match value {
            VmType::Auto => None,
            VmType::Lxc => Some(proxmox_api::VmType::Lxc),
            VmType::Qemu => Some(proxmox_api::VmType::Qemu),
        }
    }
}

#[derive(Debug)]
enum AuthArgs {
    UsernamePassword { username: String, password: String },
    Apikey { apikey: String, tokenid: String },
}

impl AuthArgs {
    fn parse(cli_args: &MainCliArgs) -> Self {
        let username = cli_args.username.as_ref().cloned().unwrap_or_default();
        let password = cli_args.password.as_ref().cloned().unwrap_or_default();
        let tokenid = cli_args.tokenid.as_ref().cloned().unwrap_or_default();
        let apikey = cli_args.apikey.as_ref().cloned().unwrap_or_default();

        if !username.is_empty() || !password.is_empty() {
            if !apikey.is_empty() || !tokenid.is_empty() {
                eprintln!("Please only provide either username and password or apikey, not both.");
                exit(1);
            }
            Self::UsernamePassword { username, password }
        } else if !apikey.is_empty() && !tokenid.is_empty() {
            Self::Apikey { apikey, tokenid }
        } else {
            eprintln!("Please provide either username and password or apikey and tokenid.");
            exit(1);
        }
    }
}
/// Test the API client via CLI.
#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();
    debug!("init logger");

    let args = MainCliArgs::parse();
    let auth = AuthArgs::parse(&args);
    if args.url.is_empty() {
        eprintln!("Please provide a URL.");
        exit(1);
    }

    let client = match auth {
        AuthArgs::UsernamePassword { username, password } => {
            ProxmoxApiClient::connect_with_ticket(
                &http::Uri::from_str(&args.url).expect("failed to parse URL"),
                &username,
                SecureString::from(password),
                args.ignore_ssl_errors,
            )
            .await
        }
        AuthArgs::Apikey { apikey, tokenid } => {
            ProxmoxApiClient::connect_with_apikey(
                &http::Uri::from_str(&args.url).expect("failed to parse URL"),
                &tokenid,
                SecureString::from(apikey),
                args.ignore_ssl_errors,
            )
            .await
        }
    }
    .expect("failed to create API client");

    match process_cmd(&client, &args.command).await {
        Ok(_) => println!("OK."),
        Err(err) => eprintln!("failed: {err:?}"),
    }
}

async fn process_cmd(client: &ProxmoxApiClient, cmd: &Command) -> proxmox_api::Result<()> {
    match cmd {
        Command::Nodes => {
            for node in client.nodes().await? {
                println!("{node:?}");
            }
        }
        Command::NodesForever => loop {
            for node in client.nodes().await? {
                println!("{node:?}");
            }
            info!("waiting 60 minutes...");
            sleep(Duration::from_secs(60 * 60)).await;
        },

        Command::ListVmsLxc { node } => {
            for vm in client.node_lxc(&NodeId::from_str(node)?).await? {
                println!("{vm:?}");
            }
        }
        Command::ListVmsQemu { node } => {
            for vm in client.node_qemu(&NodeId::from_str(node)?, true).await? {
                println!("{vm:?}");
            }
        }

        Command::RebootNode { node } => client.node_reboot(&NodeId::from_str(node)?).await?,
        Command::ShutdownNode { node } => client.node_shutdown(&NodeId::from_str(node)?).await?,

        Command::Adapters {
            node,
            vmid,
            vm_type,
        } => {
            let node_id = NodeId::from_str(node)?;
            let vm_id = VmId::from(*vmid);
            let adapters = client
                .vm_available_console_proxies(&node_id, &vm_id, (*vm_type).into())
                .await?;
            println!("adapters = {:?}", adapters.as_ref());
        }

        Command::Start {
            node,
            vmid,
            vm_type,
        } => {
            let response = client
                .vm_start(
                    &NodeId::from_str(node)?,
                    &VmId::from(*vmid),
                    (*vm_type).into(),
                    Default::default(),
                )
                .await?;
            println!("response = {:?}", response);
        }
        Command::Stop {
            node,
            vmid,
            vm_type,
        } => {
            let response = client
                .vm_stop(
                    &NodeId::from_str(node)?,
                    &VmId::from(*vmid),
                    (*vm_type).into(),
                    Default::default(),
                )
                .await?;
            println!("response = {:?}", response);
        }
        Command::Reboot {
            node,
            vmid,
            vm_type,
        } => {
            let response = client
                .vm_reboot(
                    &NodeId::from_str(node)?,
                    &VmId::from(*vmid),
                    (*vm_type).into(),
                    Default::default(),
                )
                .await?;
            println!("response = {:?}", response);
        }
        Command::Shutdown {
            node,
            vmid,
            vm_type,
        } => {
            let response = client
                .vm_shutdown(
                    &NodeId::from_str(node)?,
                    &VmId::from(*vmid),
                    (*vm_type).into(),
                    Default::default(),
                )
                .await?;
            println!("response = {:?}", response);
        }
        Command::Reset { node, vmid } => {
            let response = client
                .qemu_vm_reset(
                    &NodeId::from_str(node)?,
                    &VmId::from(*vmid),
                    Default::default(),
                )
                .await?;
            println!("response = {:?}", response);
        }
        Command::Suspend {
            node,
            vmid,
            vm_type,
        } => {
            let response = client
                .vm_suspend(
                    &NodeId::from_str(node)?,
                    &VmId::from(*vmid),
                    (*vm_type).into(),
                    Default::default(),
                )
                .await?;
            println!("response = {:?}", response);
        }

        Command::SpiceNode { node } => {
            let response = client
                .node_spiceshell(&NodeId::from_str(node)?, Default::default())
                .await?;
            println!("response = {:?}", response);
        }
        Command::VncNode { node } => {
            let response = client
                .node_vncshell(&NodeId::from_str(node)?, Default::default())
                .await?;
            println!("response = {:?}", response);
        }
        Command::TermNode { node } => {
            let response = client
                .node_termproxy(&NodeId::from_str(node)?, Default::default())
                .await?;
            println!(
                "/nodes/{}/vncwebsocket/?port={}&vncticket={}",
                node, response.port, response.ticket
            );
        }

        Command::Spice {
            node,
            vmid,
            vm_type,
        } => {
            let response = client
                .vm_spiceproxy(
                    &NodeId::from_str(node)?,
                    &VmId::from(*vmid),
                    (*vm_type).into(),
                    Default::default(),
                )
                .await?;
            println!("response = {:?}", response);
        }
        Command::Vnc {
            node,
            vmid,
            vm_type,
        } => {
            let response = client
                .vm_vncproxy(
                    &NodeId::from_str(node)?,
                    &VmId::from(*vmid),
                    (*vm_type).into(),
                    Default::default(),
                )
                .await?;
            println!("response = {:?}", response);
        }
        Command::Term {
            node,
            vmid,
            vm_type,
        } => {
            let (vm_type, response) = client
                .vm_termproxy(
                    &NodeId::from_str(node)?,
                    &VmId::from(*vmid),
                    (*vm_type).into(),
                    Default::default(),
                )
                .await?;
            let route = match vm_type {
                proxmox_api::VmType::Lxc => "lxc",
                proxmox_api::VmType::Qemu => "qemu",
            };
            println!(
                "/nodes/{}/{}/{}/vncwebsocket/?port={}&vncticket={}",
                node, route, vmid, response.port, response.ticket
            );
        }
    }
    Ok(())
}
