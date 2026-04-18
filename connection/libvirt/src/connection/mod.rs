/* Copyright 2024-2026 Marco Köpcke
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

mod adapter;
mod graphics;

use std::borrow::Cow;
use std::ops::Deref;
use std::sync::Arc;
use std::thread;

use anyhow::anyhow;
use futures::channel::oneshot;
use futures::future::LocalBoxFuture;
use futures::{StreamExt, TryStreamExt, stream};
use gettextrs::gettext;
use indexmap::IndexMap;
use log::{debug, error, warn};
use virt::connect::Connect;
use virt::domain::Domain;
use virt::sys::{
    VIR_CONNECT_LIST_DOMAINS_ACTIVE, VIR_CONNECT_LIST_DOMAINS_INACTIVE,
    VIR_DOMAIN_DESTROY_GRACEFUL, VIR_DOMAIN_PAUSED, VIR_DOMAIN_REBOOT_ACPI_POWER_BTN,
    VIR_DOMAIN_SHUTDOWN_ACPI_POWER_BTN, VIR_DOMAIN_START_PAUSED,
};

use crate::connection::graphics::LibvirtGraphics;
use libfieldmonitor::adapter::rdp::RdpAdapter;
use libfieldmonitor::adapter::spice::SpiceAdapter;
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::adapter::vte_pty::VtePtyAdapter;
use libfieldmonitor::cache::{Cached, LoadCacheObject};
use libfieldmonitor::connection::*;
use libfieldmonitor::i18n::gettext_f;
use libfieldmonitor::libexec_path;

pub const PTY_DRIVER_BIN: &str = "de.capypara.FieldMonitor.PtyDrv.Libvirt";

#[derive(Debug, Clone)]
pub(crate) struct VirtArc<T>(Arc<()>, T, Connect);

impl VirtArc<Connect> {
    pub fn new(connect: Connect) -> Self {
        Self(Arc::new(()), connect.clone(), connect)
    }

    pub fn list_all_domains(&self) -> Result<Vec<VirtArc<Domain>>, virt::error::Error> {
        Ok(self
            .1
            .list_all_domains(VIR_CONNECT_LIST_DOMAINS_ACTIVE | VIR_CONNECT_LIST_DOMAINS_INACTIVE)?
            .into_iter()
            .map(|domain| VirtArc(self.0.clone(), domain, self.2.clone()))
            .collect::<Vec<_>>())
    }
}

impl Deref for VirtArc<Domain> {
    type Target = Domain;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<T> Drop for VirtArc<T> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.0) == 1 {
            // we are the last one.
            match self.2.close() {
                Ok(0) => debug!("Libvirt connection closed"),
                Ok(i) => warn!("Closing the libvirt connection returned {i}."),
                Err(e) => error!("libvirt connection close error: {e}"),
            }
        }
    }
}

pub struct LibvirtConnection {
    id: String,
    title: String,
    hostname: String,
    connection: VirtArc<Connect>,
    icon: Cow<'static, str>,
}

impl LibvirtConnection {
    pub async fn new(
        connection_id: &str,
        hostname: &str,
        uri: &str,
        title: &str,
        icon: Cow<'static, str>,
    ) -> ConnectionResult<Self> {
        let uri = uri.to_string();
        debug!(
            "Opening libvirt connection to {uri} [hostname for adapter connections: {hostname}]"
        );
        let connection =
            run_in_thread(move || Connect::open(Some(&uri)).map_err(virt_err)).await??;
        Ok(Self {
            id: connection_id.to_string(),
            title: title.to_string(),
            hostname: hostname.to_string(),
            connection: VirtArc::new(connection),
            icon,
        })
    }
}

impl Actionable for LibvirtConnection {}

impl Connection for LibvirtConnection {
    fn metadata(&self) -> LocalBoxFuture<'_, ConnectionMetadata> {
        Box::pin(async {
            ConnectionMetadataBuilder::default()
                .title(self.title.clone())
                .icon(IconSpec::Named(self.icon.clone()))
                .build()
                .unwrap()
        })
    }

    fn servers(&self) -> LocalBoxFuture<'_, ConnectionResult<ServerMap>> {
        Box::pin(async move {
            let connection = self.connection.clone();
            let domains =
                run_in_thread(move || connection.list_all_domains().map_err(virt_err)).await??;

            let hostname = self.hostname.clone();

            // Get the servers and their titles, and then sort by titles. Due to async
            // this is currently pretty messy (sort_by_cached_key can't take the async metadata).
            // Should probably be rewritten...
            let mut servers_and_title: IndexMap<Cow<str>, (Box<dyn ServerConnection>, String)> =
                stream::iter(domains)
                    .then(|domain| {
                        let hostname_cln = hostname.clone();
                        async move {
                            let domain_cln = domain.clone();
                            let (domain_id, domain_name) = run_in_thread(move || {
                                let domain_id = domain_cln.get_uuid()?;
                                let name = domain_cln
                                    .get_name()
                                    .unwrap_or_else(|_| gettext("(Unable to load server name)"));
                                Ok((domain_id, name))
                            })
                            .await?
                            .map_err(virt_err)?;
                            let bx: Box<dyn ServerConnection> = Box::new(LibvirtServer::new(
                                &hostname_cln,
                                domain,
                                self.id.clone(),
                                domain_name,
                            ));
                            let title = bx.metadata().await.title;
                            ConnectionResult::Ok((Cow::Owned(domain_id.to_string()), (bx, title)))
                        }
                    })
                    .try_collect()
                    .await?;
            servers_and_title.sort_by_cached_key(|_, (_, title)| title.clone());
            Ok(servers_and_title
                .into_iter()
                .map(|(k, (v, _))| (k, v))
                .collect())
        })
    }
}

pub struct LibvirtServer {
    domain: VirtArc<Domain>,
    connection_name: String,
    name: String,
    state: Cached<LibVirtServerState>,
}

impl LibvirtServer {
    fn new(
        hostname: &str,
        domain: VirtArc<Domain>,
        connection_name: String,
        domain_name: String,
    ) -> Self {
        Self {
            domain: domain.clone(),
            connection_name,
            name: domain_name.clone(),
            state: Cached::new((domain, domain_name, hostname.to_string())),
        }
    }
}

impl Actionable for LibvirtServer {
    fn actions(&self) -> LocalBoxFuture<'_, Vec<(Cow<'static, str>, Cow<'static, str>)>> {
        Box::pin(async move {
            if self.state.get().await.is_active.unwrap_or_default() {
                vec![
                    ("pmreboot".into(), gettext("Reboot").into()),
                    ("pmshutdown".into(), gettext("Shutdown").into()),
                    ("reset".into(), gettext("Force Reset").into()),
                    ("poweroff".into(), gettext("Force Poweroff").into()),
                ]
            } else {
                vec![(
                    "start".into(),
                    gettext(
                        // TRANSLATORS: Verb
                        "Start",
                    )
                    .into(),
                )]
            }
        })
    }

    fn action<'a>(&self, action_id: &str) -> Option<ServerAction<'a>> {
        match action_id {
            "pmreboot" => Some(self.act_pmreboot()),
            "pmshutdown" => Some(self.act_pmshutdown()),
            "reset" => Some(self.act_reset()),
            "poweroff" => Some(self.act_poweroff()),
            "start" => Some(self.act_start()),
            _ => None,
        }
    }
}

impl LibvirtServer {
    fn act_pmreboot<'a>(&self) -> ServerAction<'a> {
        ServerAction::new(
            Box::new(self.domain.clone()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let domain = params.downcast::<VirtArc<Domain>>().unwrap();
                    Self::exec_cmd(
                        true,
                        &domain,
                        |domain| domain.reboot(VIR_DOMAIN_REBOOT_ACPI_POWER_BTN),
                        || gettext("Reboot command successfully sent to domain."),
                        |err| {
                            gettext_f(
                                // Translators: Do NOT translate the content between '{' and '}', this is a
                                // variable name.
                                "Failed to send reboot command: {err}",
                                &[("err", err.message())],
                            )
                        },
                        toov.as_ref(),
                    )
                    .await;
                })
            }),
        )
    }
    fn act_pmshutdown<'a>(&self) -> ServerAction<'a> {
        ServerAction::new(
            Box::new(self.domain.clone()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let domain = params.downcast::<VirtArc<Domain>>().unwrap();
                    Self::exec_cmd(
                        true,
                        &domain,
                        |domain| domain.shutdown_flags(VIR_DOMAIN_SHUTDOWN_ACPI_POWER_BTN),
                        || gettext("Shutdown command successfully sent to domain."),
                        |err| {
                            gettext_f(
                                // Translators: Do NOT translate the content between '{' and '}', this is a
                                // variable name.
                                "Failed to send shutdown command: {err}",
                                &[("err", err.message())],
                            )
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
            Box::new(self.domain.clone()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let domain = params.downcast::<VirtArc<Domain>>().unwrap();
                    Self::exec_cmd(
                        true,
                        &domain,
                        |domain| domain.reset(),
                        || gettext("Domain successfully reset."),
                        |err| {
                            gettext_f(
                                // Translators: Do NOT translate the content between '{' and '}', this is a
                                // variable name.
                                "Failed to reset domain: {err}",
                                &[("err", err.message())],
                            )
                        },
                        toov.as_ref(),
                    )
                    .await;
                })
            }),
        )
    }
    fn act_poweroff<'a>(&self) -> ServerAction<'a> {
        ServerAction::new(
            Box::new(self.domain.clone()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let domain = params.downcast::<VirtArc<Domain>>().unwrap();
                    Self::exec_cmd(
                        true,
                        &domain,
                        |domain| domain.destroy_flags(VIR_DOMAIN_DESTROY_GRACEFUL),
                        || gettext("Domain successfully shut down."),
                        |err| {
                            gettext_f(
                                // Translators: Do NOT translate the content between '{' and '}', this is a
                                // variable name.
                                "Failed to send destroy command: {err}",
                                &[("err", err.message())],
                            )
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
            Box::new(self.domain.clone()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let domain = params.downcast::<VirtArc<Domain>>().unwrap();
                    Self::exec_cmd(
                        false,
                        &domain,
                        |domain| {
                            if !domain.is_active().unwrap_or_default() {
                                domain.create_with_flags(VIR_DOMAIN_START_PAUSED)?;
                            }
                            domain.resume()
                        },
                        || gettext("Domain successfully started."),
                        |err| {
                            gettext_f(
                                // Translators: Do NOT translate the content between '{' and '}', this is a
                                // variable name.
                                "Failed to send create command: {err}",
                                &[("err", err.message())],
                            )
                        },
                        toov.as_ref(),
                    )
                    .await;
                })
            }),
        )
    }

    async fn exec_cmd<F, S>(
        should_be_running: bool,
        domain: &VirtArc<Domain>,
        cmd: F,
        success_msg: impl (Fn() -> String) + Send + 'static,
        err_msg: impl (Fn(virt::error::Error) -> String) + Send + 'static,
        toov: Option<&adw::ToastOverlay>,
    ) where
        F: (Fn(&VirtArc<Domain>) -> Result<S, virt::error::Error>) + Send + 'static,
        S: Send,
    {
        let domain = domain.clone();
        let text = run_in_thread(move || {
            let running = domain.is_active().unwrap_or_default();
            if should_be_running && !running {
                return gettext("Domain is not running.");
            } else if !should_be_running && running {
                let is_paused = domain
                    .get_state()
                    .map(|(s, _)| s == VIR_DOMAIN_PAUSED)
                    .unwrap_or_default();
                if !is_paused {
                    return gettext("Domain is already running.");
                }
            }
            let result = cmd(&domain);
            result.map(|_| success_msg()).unwrap_or_else(err_msg)
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
}

impl ServerConnection for LibvirtServer {
    fn metadata(&self) -> LocalBoxFuture<'_, ServerMetadata> {
        Box::pin(async move {
            ServerMetadataBuilder::default()
                .title(self.name.clone())
                .is_online(self.state.get().await.is_active)
                .build()
                .unwrap()
        })
    }

    fn supported_adapters(&self) -> LocalBoxFuture<'_, Vec<(Cow<'_, str>, Cow<'_, str>)>> {
        Box::pin(async move {
            if !self.state.get().await.is_active.unwrap_or(true) {
                return vec![];
            }
            let mut adapters = Vec::with_capacity(4);
            let graphics = &self.state.get().await.graphics;
            graphics.push_supported_adapters(&mut adapters);
            adapters.push((VtePtyAdapter::TAG.into(), gettext("Serial Console").into()));
            adapters
        })
    }

    fn create_adapter(&self, tag: &str) -> LocalBoxFuture<'_, ConnectionResult<Box<dyn Adapter>>> {
        let tag = tag.to_string();
        Box::pin(async move {
            let graphics = self.state.get().await.graphics.clone();
            let bx: Box<dyn Adapter> = match &*tag {
                SpiceAdapter::TAG => graphics.into_spice_adapter().map(Box::new)?,
                RdpAdapter::TAG => graphics.into_rdp_adapter().map(Box::new)?,
                VncAdapter::TAG => graphics.into_vnc_adapter().map(Box::new)?,
                VtePtyAdapter::TAG => {
                    let uri = self
                        .domain
                        .get_connect()
                        .and_then(|c| c.get_uri())
                        .map_err(|e| ConnectionError::General(None, e.into()))?;
                    let domid = self
                        .domain
                        .get_uuid_string()
                        .map_err(|e| ConnectionError::General(None, e.into()))?;
                    Box::new(VtePtyAdapter::new(
                        self.connection_name.clone(),
                        self.name.clone(),
                        VtePtyAdapter::TAG.to_string(),
                        libexec_path(PTY_DRIVER_BIN).expect("failed to find libvirt vte driver in path. Is Field Monitor correctly installed?"),
                        vec![uri, domid],
                    ))
                }
                tag => Err(ConnectionError::General(
                    None,
                    anyhow!("invalid / unknown tag for adapter: {tag}"),
                ))?,
            };
            Ok(bx)
        })
    }
}

struct LibVirtServerState {
    is_active: Option<bool>,
    graphics: LibvirtGraphics,
}

impl LoadCacheObject for LibVirtServerState {
    type Params = (VirtArc<Domain>, String, String);

    async fn construct(previous_value: Option<Self>, params: &Self::Params) -> Self
    where
        Self: Sized,
    {
        let (domain, domain_name, hostname) = params;
        let is_active = domain.is_active().ok();
        let is_paused = domain
            .get_state()
            .map(|(s, _)| s == VIR_DOMAIN_PAUSED)
            .unwrap_or_default();
        let is_active = is_active.map(|ia| ia && !is_paused);

        // There is no need to rebuild the graphics information if we already had it and
        // were online before.
        let graphics = match previous_value {
            // We are still (maybe) online and were online before:
            Some(prev) if is_active != Some(false) && prev.is_active != Some(false) => {
                prev.graphics
            }
            // else:
            _ => {
                if is_active != Some(false) {
                    LibvirtGraphics::new_for(hostname, domain_name, domain)
                } else {
                    LibvirtGraphics::default()
                }
            }
        };

        Self {
            is_active,
            graphics,
        }
    }
}

fn virt_err(error: virt::error::Error) -> ConnectionError {
    ConnectionError::General(Some(error.message().to_string()), error.into())
}

async fn run_in_thread<F, T>(task: F) -> ConnectionResult<T>
where
    F: (FnOnce() -> T) + Send + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = oneshot::channel();
    thread::spawn(move || {
        sender.send(task()).ok();
    });
    receiver
        .await
        .map_err(|e| ConnectionError::General(None, e.into()))
}
