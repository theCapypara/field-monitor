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
struct LibvirtGraphicsCreds {
    host: String,
    port: NonZeroU32,
    password: Option<SecureString>,
}

#[derive(Debug, Default, Clone)]
pub struct LibvirtGraphics {
    spice: Option<LibvirtGraphicsCreds>,
    vnc: Option<LibvirtGraphicsCreds>,
    rdp: Option<LibvirtGraphicsCreds>,
}

impl LibvirtGraphics {
    pub fn new_for(hostname: &str, name: &str, domain: &Domain) -> Self {
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

    pub fn into_spice_adapter(self) -> ConnectionResult<SpiceAdapter> {
        self.spice
            .map(|creds| {
                SpiceAdapter::new(
                    creds.host,
                    creds.port.into(),
                    String::new(),
                    creds.password.unwrap_or_else(|| "".into()),
                )
            })
            .ok_or_else(|| {
                ConnectionError::General(None, anyhow!("spice not supported on this domain"))
            })
    }

    pub fn into_rdp_adapter(self) -> ConnectionResult<RdpAdapter> {
        self.rdp
            .map(|creds| {
                RdpAdapter::new(
                    creds.host,
                    creds.port.into(),
                    String::new(),
                    creds.password.unwrap_or_else(|| "".into()),
                )
            })
            .ok_or_else(|| {
                ConnectionError::General(None, anyhow!("rdp not supported on this domain"))
            })
    }

    pub fn into_vnc_adapter(self) -> ConnectionResult<VncAdapter> {
        self.vnc
            .map(|creds| {
                VncAdapter::new(
                    creds.host,
                    creds.port.into(),
                    String::new(),
                    creds.password.unwrap_or_else(|| "".into()),
                )
            })
            .ok_or_else(|| {
                ConnectionError::General(None, anyhow!("vnc not supported on this domain"))
            })
    }
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
