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
use std::num::NonZeroU32;
use std::ops::Deref;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::anyhow;
use async_std::task::sleep;
use futures::channel::oneshot;
use futures::future::LocalBoxFuture;
use futures::{stream, StreamExt, TryStreamExt};
use gettextrs::gettext;
use log::{debug, error, warn};
use quick_xml::de::from_str;
use quick_xml::impl_deserialize_for_internally_tagged_enum;
use secure_string::SecureString;
use serde::Deserialize;
use virt::connect::Connect;
use virt::domain::Domain;
use virt::sys::{
    VIR_CONNECT_LIST_DOMAINS_ACTIVE, VIR_CONNECT_LIST_DOMAINS_INACTIVE,
    VIR_DOMAIN_DESTROY_GRACEFUL, VIR_DOMAIN_PAUSED, VIR_DOMAIN_REBOOT_ACPI_POWER_BTN,
    VIR_DOMAIN_SHUTDOWN_ACPI_POWER_BTN, VIR_DOMAIN_START_PAUSED, VIR_DOMAIN_XML_SECURE,
};

use libfieldmonitor::adapter::rdp::RdpAdapter;
use libfieldmonitor::adapter::spice::SpiceAdapter;
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::adapter::vte_pty::VtePtyAdapter;
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
    fn metadata(&self) -> ConnectionMetadata {
        ConnectionMetadataBuilder::default()
            .title(self.title.clone())
            .icon(IconSpec::Named(self.icon.clone()))
            .build()
            .unwrap()
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            let connection = self.connection.clone();
            let domains =
                run_in_thread(move || connection.list_all_domains().map_err(virt_err)).await??;

            let hostname = self.hostname.clone();

            let mut servers: ServerMap = stream::iter(domains.into_iter())
                .then(|domain| {
                    let hostname_cln = hostname.clone();
                    async move {
                        let domain_cln = domain.clone();
                        let (domain_id, name, is_active) = run_in_thread(move || {
                            let domain_id = domain_cln.get_uuid()?;
                            let name = domain_cln
                                .get_name()
                                .unwrap_or_else(|_| gettext("(Unable to load server name)"));
                            let is_active = domain_cln.is_active().ok();
                            let is_paused = domain_cln
                                .get_state()
                                .map(|(s, _)| s == VIR_DOMAIN_PAUSED)
                                .unwrap_or_default();
                            Ok((domain_id, name, is_active.map(|ia| ia && !is_paused)))
                        })
                        .await?
                        .map_err(virt_err)?;
                        let bx: Box<dyn ServerConnection> = Box::new(LibvirtServer::new(
                            &hostname_cln,
                            domain,
                            self.id.clone(),
                            name,
                            is_active,
                        ));
                        Ok((Cow::Owned(domain_id.to_string()), bx))
                    }
                })
                .try_collect()
                .await?;

            servers.sort_by_cached_key(|_, srv| srv.metadata().title);

            Ok(servers)
        })
    }
}

#[derive(Debug, Clone)]
struct LibvirtGraphicsCreds {
    host: String,
    port: NonZeroU32,
    password: Option<SecureString>,
}

#[derive(Debug, Default, Clone)]
struct LibvirtGraphics {
    spice: Option<LibvirtGraphicsCreds>,
    vnc: Option<LibvirtGraphicsCreds>,
    rdp: Option<LibvirtGraphicsCreds>,
}

#[derive(Debug)]
enum LibvirtXmlGraphics {
    Vnc {
        port: Option<i64>,
        passwd: Option<String>,
    },
    Rdp {
        port: Option<i64>,
        passwd: Option<String>,
    },
    Spice {
        port: Option<i64>,
        passwd: Option<String>,
    },
    Other,
}

impl_deserialize_for_internally_tagged_enum! {
    LibvirtXmlGraphics, "@type",
    ("vnc"    => Vnc {
        #[serde(rename = "@port", default)]
        port: Option<i64>,
        #[serde(rename = "@passwd", default)]
        passwd: Option<String>,
    }),
    ("rdp" => Rdp {
        #[serde(rename = "@port", default)]
        port: Option<i64>,
        #[serde(rename = "@passwd", default)]
        passwd: Option<String>,
    }),
    ("spice" => Spice {
        #[serde(rename = "@port", default)]
        port: Option<i64>,
        #[serde(rename = "@passwd", default)]
        passwd: Option<String>,
    }),
    (_ => Other),
}

#[derive(Debug, Deserialize)]
struct LibvirtXmlDevices {
    graphics: Vec<LibvirtXmlGraphics>,
}

#[derive(Debug, Deserialize)]
struct LibvirtXmlDomain {
    devices: LibvirtXmlDevices,
}

pub struct LibvirtServer {
    domain: VirtArc<Domain>,
    is_active: Option<bool>,
    connection_name: String,
    name: String,
    graphics: LibvirtGraphics,
}

impl LibvirtServer {
    fn new(
        hostname: &str,
        domain: VirtArc<Domain>,
        connection_name: String,
        name: String,
        is_active: Option<bool>,
    ) -> Self {
        Self {
            graphics: if is_active.unwrap_or(true) {
                Self::graphics_for(hostname, &name, &domain)
            } else {
                LibvirtGraphics::default()
            },
            domain,
            connection_name,
            name,
            is_active,
        }
    }

    fn graphics_for(hostname: &str, name: &str, domain: &Domain) -> LibvirtGraphics {
        debug!("loading graphics options for {name}");
        let xml_str = match domain.get_xml_desc(VIR_DOMAIN_XML_SECURE) {
            Ok(xml) => xml,
            Err(err) => {
                error!("Failed to load XML description for {name}: {err}");
                return LibvirtGraphics::default();
            }
        };
        let xml: LibvirtXmlDomain = match from_str(&xml_str) {
            Ok(xml) => {
                debug!("libvirt xml: {xml:?}");
                xml
            }
            Err(err) => {
                error!("Failed to deserialize XML description for {name}: {err}");
                return LibvirtGraphics::default();
            }
        };
        let mut graphics = LibvirtGraphics::default();

        fn map_graphics(
            host: &str,
            inp: &LibvirtXmlGraphics,
            out: &mut LibvirtGraphics,
        ) -> Result<(), String> {
            let built = LibvirtGraphicsCreds {
                host: host.to_string(),
                port: get_port(inp)?,
                password: get_passwd(inp)?,
            };
            match inp {
                LibvirtXmlGraphics::Vnc { .. } => {
                    out.vnc = Some(built);
                }
                LibvirtXmlGraphics::Rdp { .. } => {
                    out.rdp = Some(built);
                }
                LibvirtXmlGraphics::Spice { .. } => {
                    out.spice = Some(built);
                }
                LibvirtXmlGraphics::Other => {}
            }
            Ok(())
        }

        // TODO: I guess we COULD try and figure out if the given address on the connection would even
        //       allow us to connect, but it's pretty complicated with plenty failure / edge cases
        //       to properly detect, and in most cases would probably just be confusing for the end
        //       user, if they just see no option to connect.
        let host = hostname;

        for inp in &xml.devices.graphics {
            if let Err(err) = map_graphics(host, inp, &mut graphics) {
                warn!("failed to process graphics entry for {name}: {err}");
            }
        }

        debug!("Libvirt server {name} graphics connection info: {graphics:?}");
        graphics
    }
}

impl Actionable for LibvirtServer {
    fn actions(&self) -> Vec<(Cow<'static, str>, Cow<'static, str>)> {
        if self.is_active.unwrap_or_default() {
            vec![
                ("pmreboot".into(), gettext("Reboot").into()),
                ("pmshutdown".into(), gettext("Shutdown").into()),
                ("reset".into(), gettext("Force Reset").into()),
                ("poweroff".into(), gettext("Force Poweroff").into()),
            ]
        } else {
            vec![("start".into(), gettext("Start / Resume").into())]
        }
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
                    let (success, force_reload) = Self::exec_cmd(
                        true,
                        &domain,
                        |domain| domain.reboot(VIR_DOMAIN_REBOOT_ACPI_POWER_BTN),
                        || gettext("Reboot command successfully sent to domain."),
                        |err| {
                            gettext_f(
                                "Failed to send reboot command: {err}",
                                &[("err", err.message())],
                            )
                        },
                        toov.as_ref(),
                    )
                    .await;
                    success || force_reload
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
                    let (success, force_reload) = Self::exec_cmd(
                        true,
                        &domain,
                        |domain| domain.shutdown_flags(VIR_DOMAIN_SHUTDOWN_ACPI_POWER_BTN),
                        || gettext("Shutdown command successfully sent to domain."),
                        |err| {
                            gettext_f(
                                "Failed to send shutdown command: {err}",
                                &[("err", err.message())],
                            )
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
            Box::new(self.domain.clone()),
            Box::new(|params, _window, toov| {
                Box::pin(async move {
                    let domain = params.downcast::<VirtArc<Domain>>().unwrap();
                    let (success, force_reload) = Self::exec_cmd(
                        true,
                        &domain,
                        |domain| domain.reset(),
                        || gettext("Domain successfully reset."),
                        |err| gettext_f("Failed to reset domain: {err}", &[("err", err.message())]),
                        toov.as_ref(),
                    )
                    .await;

                    if success {
                        // Sleep shortly to allow hypervisor to (ideally) have restarted the domain.
                        sleep(Duration::from_millis(500)).await;
                    }

                    success || force_reload
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
                    let (success, force_reload) = Self::exec_cmd(
                        true,
                        &domain,
                        |domain| domain.destroy_flags(VIR_DOMAIN_DESTROY_GRACEFUL),
                        || gettext("Domain successfully shut down."),
                        |err| {
                            gettext_f(
                                "Failed to send destroy command: {err}",
                                &[("err", err.message())],
                            )
                        },
                        toov.as_ref(),
                    )
                    .await;

                    if success {
                        // Sleep shortly to allow hypervisor to (ideally) have shutdown the domain.
                        sleep(Duration::from_millis(500)).await;
                    }

                    success || force_reload
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
                    let (success, force_reload) = Self::exec_cmd(
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
                                "Failed to send create command: {err}",
                                &[("err", err.message())],
                            )
                        },
                        toov.as_ref(),
                    )
                    .await;

                    if success {
                        // Sleep shortly to allow hypervisor to (ideally) have started the domain.
                        sleep(Duration::from_millis(500)).await;
                    }

                    success || force_reload
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
    ) -> (bool, bool)
    where
        F: (Fn(&VirtArc<Domain>) -> Result<S, virt::error::Error>) + Send + 'static,
        S: Send,
    {
        let domain = domain.clone();
        let (success, should_reload, text) = run_in_thread(move || {
            let running = domain.is_active().unwrap_or_default();
            if should_be_running && !running {
                return (false, true, gettext("Domain is not running."));
            } else if !should_be_running && running {
                let is_paused = domain
                    .get_state()
                    .map(|(s, _)| s == VIR_DOMAIN_PAUSED)
                    .unwrap_or_default();
                if !is_paused {
                    return (false, true, gettext("Domain is already running."));
                }
            }
            let result = cmd(&domain);
            (
                result.is_ok(),
                false,
                result.map(|_| success_msg()).unwrap_or_else(err_msg),
            )
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

        if let Some(toov) = toov {
            toov.add_toast(adw::Toast::builder().title(&text).timeout(5).build());
        }
        (success, should_reload)
    }
}

impl ServerConnection for LibvirtServer {
    fn metadata(&self) -> ServerMetadata {
        ServerMetadataBuilder::default()
            .title(self.name.clone())
            .is_online(self.is_active)
            .build()
            .unwrap()
    }

    fn supported_adapters(&self) -> Vec<(Cow<str>, Cow<str>)> {
        if !self.is_active.unwrap_or(true) {
            return vec![];
        }
        let mut adapters = Vec::with_capacity(4);
        if self.graphics.spice.is_some() {
            adapters.push((
                SpiceAdapter::TAG.into(),
                gettext("SPICE (Graphical)").into(),
            ));
        }
        if self.graphics.rdp.is_some() {
            adapters.push((RdpAdapter::TAG.into(), gettext("RDP (Graphical)").into()));
        }
        if self.graphics.vnc.is_some() {
            adapters.push((VncAdapter::TAG.into(), gettext("VNC (Graphical)").into()));
        }
        adapters.push((VtePtyAdapter::TAG.into(), gettext("Serial Console").into()));
        adapters
    }

    fn create_adapter(&self, tag: &str) -> LocalBoxFuture<ConnectionResult<Box<dyn Adapter>>> {
        let tag = tag.to_string();
        let graphics = self.graphics.clone();
        Box::pin(async move {
            let bx: Box<dyn Adapter> = match &*tag {
                SpiceAdapter::TAG => {
                    if let Some(creds) = graphics.spice {
                        Box::new(SpiceAdapter::new(
                            creds.host,
                            creds.port.into(),
                            String::new(),
                            creds.password.unwrap_or_else(|| "".into()),
                        ))
                    } else {
                        Err(ConnectionError::General(
                            None,
                            anyhow!("spice not supported on this domain"),
                        ))?
                    }
                }
                RdpAdapter::TAG => {
                    if let Some(creds) = graphics.rdp {
                        Box::new(RdpAdapter::new(
                            creds.host,
                            creds.port.into(),
                            String::new(),
                            creds.password.unwrap_or_else(|| "".into()),
                        ))
                    } else {
                        Err(ConnectionError::General(
                            None,
                            anyhow!("rdp not supported on this domain"),
                        ))?
                    }
                }
                VncAdapter::TAG => {
                    if let Some(creds) = graphics.vnc {
                        Box::new(VncAdapter::new(
                            creds.host,
                            creds.port.into(),
                            String::new(),
                            creds.password.unwrap_or_else(|| "".into()),
                        ))
                    } else {
                        Err(ConnectionError::General(
                            None,
                            anyhow!("vnc not supported on this domain"),
                        ))?
                    }
                }
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

fn get_port(graphics: &LibvirtXmlGraphics) -> Result<NonZeroU32, String> {
    let port_opt = match graphics {
        LibvirtXmlGraphics::Vnc { port, .. } => port,
        LibvirtXmlGraphics::Rdp { port, .. } => port,
        LibvirtXmlGraphics::Spice { port, .. } => port,
        LibvirtXmlGraphics::Other => return Err("unsupported graphics type".to_string()),
    };
    let Some(port_i64) = *port_opt else {
        return Err("port missing".to_string());
    };
    let Ok(port) = u32::try_from(port_i64).and_then(NonZeroU32::try_from) else {
        return Err("invalid port".to_string());
    };
    Ok(port)
}

fn get_passwd(graphics: &LibvirtXmlGraphics) -> Result<Option<SecureString>, String> {
    let passwd_opt = match graphics {
        LibvirtXmlGraphics::Vnc { passwd, .. } => passwd,
        LibvirtXmlGraphics::Rdp { passwd, .. } => passwd,
        LibvirtXmlGraphics::Spice { passwd, .. } => passwd,
        LibvirtXmlGraphics::Other => return Err("unsupported graphics type".to_string()),
    };
    Ok(passwd_opt.as_ref().map(SecureString::from))
}
