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

use crate::connection::VirtArc;
use crate::connection::adapter::LibvirtDynamicAdapter;
use crate::is_localhost;
use anyhow::anyhow;
use gettextrs::gettext;
use libfieldmonitor::adapter::rdp::RdpAdapter;
use libfieldmonitor::adapter::spice::SpiceAdapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::connection::{ConnectionError, ConnectionResult};
use log::{debug, error, warn};
use quick_xml::de::from_str;
use quick_xml::impl_deserialize_for_internally_tagged_enum;
use secure_string::SecureString;
use serde::Deserialize;
use std::borrow::Cow;
use std::num::NonZeroU32;
use virt::domain::Domain;
use virt::sys::VIR_DOMAIN_XML_SECURE;

#[derive(Debug, Clone)]
pub struct LibvirtSocketCreds {
    pub socket: String,
    pub username: Option<String>,
    pub password: Option<SecureString>,
}

#[derive(Debug, Clone)]
pub struct LibvirtNetworkCreds {
    pub host: String,
    // guaranteed to be set for RDP, VNC
    pub port: Option<NonZeroU32>,
    // guaranteed to be none for RDP, VNC
    pub tls_port: Option<NonZeroU32>,
    pub username: Option<String>,
    pub password: Option<SecureString>,
}

#[derive(Clone, Debug, Default)]
/// Information on how to connect to a graphics endpoint
pub struct LibvirtConnectable {
    // Connect via socket, path is socket address
    pub via_socket: Option<LibvirtSocketCreds>,
    // Connect via fd, id is the index of the graphics element
    pub via_fd: Option<(VirtArc<Domain>, usize)>,
    // Connect via network
    pub via_network: Option<LibvirtNetworkCreds>,
}

impl LibvirtConnectable {
    fn make(
        domain: &VirtArc<Domain>,
        host: &str,
        idx: usize,
        listens: &[LibvirtXmlGraphicsListen],
        (username, password): (Option<&str>, Option<&str>),
        (tls_port, port): (Option<i64>, Option<i64>),
    ) -> LibvirtConnectable {
        let password = password.map(SecureString::from);
        debug!("building LibvirtConnectable: {idx}");
        let mut slf = Self::default();

        let mut network_connectable = false;
        let mut socket_connectable: Option<String> = None;
        for listen in listens {
            debug!("- listen: {listen:?}");
            match listen {
                LibvirtXmlGraphicsListen::Address { address } => {
                    network_connectable = true;
                    if address.as_deref() != Some("0.0.0.0") {
                        // TODO: we are currently NOT connecting to the server via the ssh tunnel, we are always connecting locally
                        //       to the server using its hostname. If the address is reported as anything other than 0.0.0.0
                        //       this may not work, but we'll try anyway.
                        warn!(
                            "address for libvirt graphics listen address was not 0.0.0.0. connection may not be possible (ssh tunnel is not used)."
                        )
                    }
                }
                LibvirtXmlGraphicsListen::Network { network, address } => {
                    network_connectable = true;
                    // TODO: See comment above
                    warn!(
                        "libvirt graphics element has 'network' listen element (net: {network:?}, addr: {address:?}. trying to connect to the display may not be possible (ssh tunnel not used)."
                    )
                }
                LibvirtXmlGraphicsListen::Socket { socket } => {
                    socket_connectable = socket.clone();
                }
                _ => {}
            }
        }

        // If we are on localhost, add the fd option, no matter what.
        if is_localhost(host) {
            debug!("-> option fd: idx: {idx}");
            slf.via_fd = Some((domain.clone(), idx));
            // TODO: If we used the ssh tunnel we may be able to connect to the socket? not sure actually.
            if let Some(socket) = socket_connectable {
                debug!("-> option socket: {username:?}@{password:?} via {socket}");
                slf.via_socket = Some(LibvirtSocketCreds {
                    socket,
                    username: username.map(ToOwned::to_owned),
                    password: password.clone(),
                });
            }
        }

        if network_connectable {
            let port = port.and_then(parse_port);
            let tls_port = tls_port.and_then(parse_port);
            if tls_port.is_some() || port.is_some() {
                debug!(
                    "-> option network: {username:?}@{password:?} via {host}:{port:?} (tls {tls_port:?})"
                );
                slf.via_network = Some(LibvirtNetworkCreds {
                    host: host.to_string(),
                    port,
                    tls_port,
                    username: username.map(ToOwned::to_owned),
                    password: password.clone(),
                });
            }
        }

        slf
    }
}

#[derive(Debug, Default, Clone)]
pub struct LibvirtGraphics {
    spice: Option<LibvirtConnectable>,
    vnc: Option<LibvirtConnectable>,
    rdp: Option<LibvirtConnectable>,
}

impl LibvirtGraphics {
    pub fn new_for(hostname: &str, name: &str, domain: &VirtArc<Domain>) -> Self {
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

        for (idx, graphic) in xml.devices.graphics.iter().enumerate() {
            if let Err(err) = Self::map_graphics(domain, hostname, idx, graphic, &mut graphics) {
                warn!("failed to process graphics entry for {name}: {err}");
            }
        }

        debug!("Libvirt server {name} graphics connection info: {graphics:?}");
        graphics
    }

    fn map_graphics(
        domain: &VirtArc<Domain>,
        host: &str,
        idx: usize,
        inp: &LibvirtXmlGraphics,
        out: &mut LibvirtGraphics,
    ) -> Result<(), String> {
        match inp {
            LibvirtXmlGraphics::Vnc {
                listen,
                port,
                passwd,
            } => {
                debug!("building libvirt vnc connectable ({idx})");
                out.vnc = Some(LibvirtConnectable::make(
                    domain,
                    host,
                    idx,
                    listen,
                    (None, passwd.as_deref()),
                    (None, *port),
                ));
            }
            LibvirtXmlGraphics::Rdp {
                listen,
                username,
                passwd,
                port,
            } => {
                debug!("building libvirt rdp connectable ({idx})");
                out.rdp = Some(LibvirtConnectable::make(
                    domain,
                    host,
                    idx,
                    listen,
                    (username.as_deref(), passwd.as_deref()),
                    (None, *port),
                ));
            }
            LibvirtXmlGraphics::Spice {
                listen,
                port,
                tls_port,
                passwd,
            } => {
                debug!("building libvirt spice connectable ({idx})");
                out.spice = Some(LibvirtConnectable::make(
                    domain,
                    host,
                    idx,
                    listen,
                    (None, passwd.as_deref()),
                    (*tls_port, *port),
                ));
            }
            LibvirtXmlGraphics::Other => {}
        }
        Ok(())
    }

    pub fn push_supported_adapters(&self, adapters: &mut Vec<(Cow<str>, Cow<str>)>) {
        if self.spice.is_some() {
            adapters.push((SpiceAdapter::TAG.into(), gettext("SPICE").into()));
        }
        if self.rdp.is_some() {
            adapters.push((RdpAdapter::TAG.into(), gettext("RDP").into()));
        }
        if self.vnc.is_some() {
            adapters.push((VncAdapter::TAG.into(), gettext("VNC").into()));
        }
    }

    pub fn into_spice_adapter(self) -> ConnectionResult<LibvirtDynamicAdapter<SpiceAdapter>> {
        self.spice.map(LibvirtDynamicAdapter::new).ok_or_else(|| {
            ConnectionError::General(None, anyhow!("spice not supported on this domain"))
        })
    }

    pub fn into_vnc_adapter(self) -> ConnectionResult<LibvirtDynamicAdapter<VncAdapter>> {
        self.vnc.map(LibvirtDynamicAdapter::new).ok_or_else(|| {
            ConnectionError::General(None, anyhow!("vnc not supported on this domain"))
        })
    }

    pub fn into_rdp_adapter(self) -> ConnectionResult<RdpAdapter> {
        // RDP does not support fd/socket connections.
        self.rdp
            .and_then(|c| c.via_network)
            .map(|creds| {
                RdpAdapter::new(
                    creds.host,
                    creds.port.unwrap().into(),
                    creds.username.unwrap_or_else(|| "".into()),
                    creds.password.unwrap_or_else(|| "".into()),
                )
            })
            .ok_or_else(|| {
                ConnectionError::General(None, anyhow!("rdp not supported on this domain"))
            })
    }
}

#[derive(Debug)]
// Only supported by vnc, spice, rdp
enum LibvirtXmlGraphicsListen {
    Address {
        address: Option<String>,
    },
    Network {
        network: Option<String>,
        address: Option<String>,
    },
    // Only supported by vnc, spice
    Socket {
        socket: Option<String>,
    },
    // Only supported by vnc, spice
    None,
    Other,
}

impl_deserialize_for_internally_tagged_enum! {
    LibvirtXmlGraphicsListen, "@type",
    ("address"    => Address {
        #[serde(rename = "@address", default)]
        address: Option<String>,
    }),
    ("network"    => Network {
        #[serde(rename = "@network", default)]
        network: Option<String>,
        #[serde(rename = "@address", default)]
        address: Option<String>,
    }),
    ("socket"    => Socket {
        #[serde(rename = "@socket", default)]
        socket: Option<String>,
    }),
    ("none"    => None),
    (_ => Other),
}

#[derive(Debug)]
enum LibvirtXmlGraphics {
    Vnc {
        port: Option<i64>,
        passwd: Option<String>,
        listen: Vec<LibvirtXmlGraphicsListen>,
    },
    Rdp {
        port: Option<i64>,
        username: Option<String>,
        passwd: Option<String>,
        listen: Vec<LibvirtXmlGraphicsListen>,
    },
    Spice {
        port: Option<i64>,
        tls_port: Option<i64>,
        passwd: Option<String>,
        listen: Vec<LibvirtXmlGraphicsListen>,
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
        listen: Vec<LibvirtXmlGraphicsListen>
    }),
    ("rdp" => Rdp {
        #[serde(rename = "@port", default)]
        port: Option<i64>,
        #[serde(rename = "@username", default)]
        username: Option<String>,
        #[serde(rename = "@passwd", default)]
        passwd: Option<String>,
        listen: Vec<LibvirtXmlGraphicsListen>
    }),
    ("spice" => Spice {
        #[serde(rename = "@port", default)]
        port: Option<i64>,
        #[serde(rename = "@tlsPort", default)]
        tls_port: Option<i64>,
        #[serde(rename = "@passwd", default)]
        passwd: Option<String>,
        listen: Vec<LibvirtXmlGraphicsListen>
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

fn parse_port(candidate: i64) -> Option<NonZeroU32> {
    u32::try_from(candidate)
        .and_then(NonZeroU32::try_from)
        .inspect_err(|_| warn!("failed reading port in libvirt xml"))
        .ok()
}
