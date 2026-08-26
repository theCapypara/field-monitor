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
use crate::connection::util::open_libvirt_fd_stream;
use crate::is_localhost;
use anyhow::anyhow;
use futures::TryFutureExt;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use libfieldmonitor::adapter::qemu_dbus::QemuDbusAdapter;
use libfieldmonitor::adapter::types::{Adapter, AdapterDisplay, NullAdapterDisplay};
use libfieldmonitor::cert_security::{VerifyTls, VerifyTlsResponse};
use libfieldmonitor::connection::ConnectionError;
use libfieldmonitor::i18n::gettext_f;
use log::{debug, error};
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use nix::unistd::{Gid, Group};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::rc::Rc;
use std::{fs, io};
use virt::domain::Domain;
use zbus::Error;
use zbus::address::transport::UnixSocket;

#[derive(Debug, Clone)]
pub(super) struct LibvirtDbusConnectable {
    domain: VirtArc<Domain>,
    bus_address: Option<String>,
    // If Some, the index of the graphics device to use for p2p connection - if None: not supported
    p2p_graphics_idx: Option<usize>,
}

impl LibvirtDbusConnectable {
    pub fn try_make(
        domain: &VirtArc<Domain>,
        host: &str,
        mut bus_address: Option<&str>,
        p2p_supported: bool,
        graphics_idx: usize,
    ) -> Option<Self> {
        // TODO: Currently we don't support remote hosts (we probably could via SSH tunnel)
        if !is_localhost(host) {
            bus_address = None;
        }
        if !p2p_supported && bus_address.is_none() {
            debug!("giving up, p2p not supported and have no adddress / or is remote");
            None
        } else {
            Some(Self {
                domain: domain.clone(),
                bus_address: bus_address.map(ToString::to_string),
                p2p_graphics_idx: if p2p_supported {
                    Some(graphics_idx)
                } else {
                    None
                },
            })
        }
    }
}

pub(super) struct LibvirtQemuDbusAdapter(LibvirtDbusConnectable);

impl LibvirtQemuDbusAdapter {
    pub fn new(connectable: LibvirtDbusConnectable) -> Self {
        Self(connectable)
    }

    async fn try_via_p2p(
        self: Box<Self>,
        error_hints: Vec<String>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
        verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> Result<Box<dyn AdapterDisplay>, (Box<Self>, Vec<String>)> {
        debug!("trying dbus via p2p");
        let Some(graphics_idx) = self.0.p2p_graphics_idx else {
            debug!("p2p not supported");
            return Err((self, error_hints));
        };
        let Ok(stream) = open_libvirt_fd_stream(&self.0.domain, graphics_idx) else {
            return Err((self, error_hints));
        };

        // create d-bus connection
        let creds_result = getsockopt(&stream, PeerCredentials);
        let mut builder = zbus::connection::Builder::async_io_unix_stream(stream).p2p();
        if let Ok(creds) = creds_result {
            builder = builder.user_id(creds.uid())
        };
        let Ok(conn) = builder
            .build()
            .await
            .inspect_err(|err| error!("failed to connect via d-bus using p2p: {err:?}"))
        else {
            return Err((self, error_hints));
        };

        Ok(Box::new(QemuDbusAdapter::new(conn, None))
            .create_and_connect_display(on_connected, on_disconnected, verify_tls)
            .await)
    }

    async fn try_via_address(
        self: Box<Self>,
        mut error_hints: Vec<String>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
        verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> Result<Box<dyn AdapterDisplay>, (Box<Self>, Vec<String>)> {
        // if the user runs into any issues here, they are really just better off
        // enabling p2p, if they can.
        error_hints.push(gettext(
            "Try enabling the 'p2p' option for the D-Bus graphics adapter.",
        ));

        debug!("trying dbus via bus address");
        let Some(bus_address) = &self.0.bus_address else {
            debug!("via address not supported");
            return Err((self, error_hints));
        };

        let connect = &self.0.domain.2;
        // not the nicest check, but good enough for this heuristic help message.
        if connect.get_uri().unwrap_or_default().ends_with("system") {
            error_hints.push(gettext(
                "If you are connecting to a qemu:///system domain, you may not be able to connect via D-Bus unless you enable the 'p2p' option."
            ));
        }

        // create d-bus connection
        let Ok(bus_address): Result<zbus::Address, _> = bus_address
            .as_str()
            .try_into()
            .inspect_err(|err| error!("failed to parse D-Bus address ({bus_address}): {err:?}"))
        else {
            return Err((self, error_hints));
        };
        let conn = match zbus::connection::Builder::address(bus_address.clone())
            .unwrap() // this is infallible since bus_address is already an Address
            .build()
            .await
        {
            Ok(conn) => conn,
            Err(err) => {
                error!("failed to connect via d-bus: {err:?}");
                if let Error::Connection(io_err, _) = &err
                    && io_err.kind() == io::ErrorKind::PermissionDenied
                {
                    let group_hint = dbus_address_group_hint(&bus_address).unwrap_or_default();
                    error_hints.push(gettext_f(
                        // Translators: Do NOT translate the content between '{' and '}', this is a
                        // variable name.
                        "Your user account does not have permission to connect to the bus.{group_hint}",
                        &[("group_hint", &group_hint)]
                    ))
                };
                return Err((self, error_hints));
            }
        };

        Ok(Box::new(QemuDbusAdapter::new(conn, None))
            .create_and_connect_display(on_connected, on_disconnected, verify_tls)
            .await)
    }

    fn not_supported(
        self,
        error_hints: Vec<String>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
    ) -> Box<dyn AdapterDisplay> {
        error!("Failed to connect via d-bus");
        let extra_info = match error_hints.len() {
            0 => String::new(),
            1 => format!("\n{}", error_hints[0]),
            _ => format!(
                "\n{}\n{}",
                gettext("The following information may help you to troubleshoot the issue:"),
                error_hints
                    .into_iter()
                    .map(|e| format!("- {e}"))
                    .collect::<Box<[_]>>()
                    .join("\n")
            ),
        };
        on_disconnected(Err(ConnectionError::General(
            Some(gettext_f(
                // Translators: Do NOT translate the content between '{' and '}', this is a
                // variable name.
                "Failed to connect via D-Bus.{extra_info}",
                &[("extra_info", &extra_info)],
            )),
            anyhow!("Failed to connect via d-bus"),
        )));
        Box::new(NullAdapterDisplay)
    }
}

impl Adapter for LibvirtQemuDbusAdapter {
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
        verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> LocalBoxFuture<'static, Box<dyn AdapterDisplay>> {
        let on_connected2 = on_connected.clone();
        let on_disconnected2 = on_disconnected.clone();
        let on_disconnected3 = on_disconnected.clone();
        let verify_tls2 = verify_tls.clone();
        let errors = vec![];
        Box::pin(
            self.try_via_p2p(errors, on_connected, on_disconnected, verify_tls)
                .or_else(|(slf, errors)| {
                    slf.try_via_address(errors, on_connected2, on_disconnected2, verify_tls2)
                })
                .unwrap_or_else(|(slf, errors)| slf.not_supported(errors, on_disconnected3)),
        )
    }
}

fn dbus_address_group_hint(bus_address: &zbus::Address) -> Option<String> {
    // The user has no permission to connect to the bus, let's help them by trying
    // to give them the group they need to be part of (if we can).
    if let zbus::address::Transport::Unix(unix_transport) = bus_address.transport() {
        let path: Option<&Path> = match unix_transport.path() {
            UnixSocket::File(path) => Some(path),
            UnixSocket::Dir(path) => Some(path),
            UnixSocket::TmpDir(path) => Some(path),
            _ => None,
        };
        path.and_then(path_group_name).map(|group_name| {
            format!(
                " {}",
                gettext_f(
                    // Translators: Do NOT translate the content between '{' and '}', this is a
                    // variable name.
                    "Try adding your account to the '{group_name}' group.",
                    &[("group_name", &group_name)],
                )
            )
        })
    } else {
        None
    }
}

/// Try to get the group name of the given path, if that fails retry with the parent path.
fn path_group_name(path: &Path) -> Option<String> {
    path_metadata(path)
        .ok()
        .or_else(|| path.parent().and_then(|parent| path_metadata(parent).ok()))
        .and_then(|metadata| {
            Group::from_gid(Gid::from_raw(metadata.gid()))
                .ok()
                .flatten()
        })
        .map(|group| group.name)
}

/// Get the metadata of the given path or if this fails, retry with the DirEntry instead
fn path_metadata(path: &Path) -> Result<fs::Metadata, io::Error> {
    fs::metadata(path).or_else(|err| {
        if let Some(parent) = path.parent()
            && let Some(file_name) = path.file_name()
        {
            for entry in fs::read_dir(parent)? {
                let entry = entry?;
                if entry.file_name() == file_name {
                    return entry.metadata();
                }
            }
        }

        Err(err)
    })
}
