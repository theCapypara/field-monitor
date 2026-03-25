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
use crate::connection::ConnectionError;
use crate::connection::graphics::LibvirtConnectable;
use anyhow::anyhow;
use gettextrs::gettext;
use gtk::glib;
use libfieldmonitor::adapter::spice::SpiceAdapter;
use libfieldmonitor::adapter::types::{Adapter, AdapterDisplay, NullAdapterDisplay};
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::cert_security::{VerifyTls, VerifyTlsResponse};
use log::{debug, error};
use std::marker::PhantomData;
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixStream;
use std::rc::Rc;
use virt::sys::VIR_DOMAIN_OPEN_GRAPHICS_SKIPAUTH;

/// A wrapper around a base adapter that tries to connect:
/// - first via direct FD connection
/// - then via socket
/// - lastly via network
pub struct LibvirtDynamicAdapter<T>(LibvirtConnectable, PhantomData<T>);

impl<T> LibvirtDynamicAdapter<T> {
    pub fn new(connectable: LibvirtConnectable) -> Self {
        Self(connectable, Default::default())
    }
}

impl LibvirtDynamicAdapter<SpiceAdapter> {
    fn try_via_fd(
        mut self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
        verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> Result<Box<dyn AdapterDisplay>, Box<Self>> {
        debug!("trying fd");
        if let Some((domain, graphics_idx)) = self.0.via_fd.take() {
            let stream = match domain
                .open_graphics_fd(graphics_idx as _, VIR_DOMAIN_OPEN_GRAPHICS_SKIPAUTH)
            {
                Ok(fd) => {
                    // SAFETY: If open_graphics_fd doesn't error, the fd points to a valid file descriptor.
                    unsafe { UnixStream::from_raw_fd(fd as _) }
                }
                Err(err) => {
                    error!("libvirt openGraphicsFd failed: {err}");
                    return Err(self);
                }
            };

            // connect the other end to the spice adapter
            Ok(Box::new(SpiceAdapter::new_from_socket(
                stream,
                Box::new(glib::clone!(
                    #[strong]
                    domain,
                    move || domain
                        .open_graphics_fd(graphics_idx as _, VIR_DOMAIN_OPEN_GRAPHICS_SKIPAUTH)
                        .map(|fd_num| {
                            // SAFETY: If open_graphics_fd doesn't error, the fd points to a valid file descriptor.
                            unsafe { UnixStream::from_raw_fd(fd_num as _) }
                        })
                        .map_err(Into::into)
                )),
                // we use skipauth, so no credentials needed.
                None,
                None,
            ))
            .create_and_connect_display(on_connected, on_disconnected, verify_tls))
        } else {
            debug!("fd not available");
            Err(self)
        }
    }

    fn try_via_socket(
        mut self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
        verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> Result<Box<dyn AdapterDisplay>, Box<Self>> {
        debug!("trying socket");
        if let Some(socket_creds) = self.0.via_socket.take() {
            let socket_addr = socket_creds.socket;
            let Ok(socket) = UnixStream::connect(&socket_addr) else {
                error!("connection to socket failed");
                return Err(self);
            };
            Ok(Box::new(SpiceAdapter::new_from_socket(
                socket,
                Box::new(glib::clone!(
                    #[strong]
                    socket_addr,
                    move || UnixStream::connect(&socket_addr).map_err(Into::into)
                )),
                socket_creds.username,
                socket_creds.password,
            ))
            .create_and_connect_display(on_connected, on_disconnected, verify_tls))
        } else {
            debug!("socket not available");
            Err(self)
        }
    }

    fn try_via_network(
        mut self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
        verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> Result<Box<dyn AdapterDisplay>, Box<Self>> {
        debug!("trying network");
        let network_cfg = self.0.via_network.take();
        if let Some(creds) = network_cfg {
            Ok(Box::new(SpiceAdapter::new(
                creds.host,
                creds.port,
                creds.tls_port,
                creds.username.unwrap_or_else(|| "".into()),
                creds.password.unwrap_or_else(|| "".into()),
            ))
            .create_and_connect_display(on_connected, on_disconnected, verify_tls))
        } else {
            debug!("network not available");
            Err(self)
        }
    }

    fn not_supported(
        self,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
    ) -> Box<dyn AdapterDisplay> {
        error!("No supported graphics endpoint available");
        on_disconnected(Err(ConnectionError::General(
            Some(gettext(
                "No supported graphics endpoint available. Tip: If connected via SSH, make sure that the graphics adapter's listen type is set to 'address' and that the adapter is listening on all interfaces.",
            )),
            anyhow!("No supported graphics endpoint available"),
        )));
        Box::new(NullAdapterDisplay)
    }
}

impl Adapter for LibvirtDynamicAdapter<SpiceAdapter> {
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
        verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> Box<dyn AdapterDisplay> {
        self.try_via_fd(
            on_connected.clone(),
            on_disconnected.clone(),
            verify_tls.clone(),
        )
        .or_else(|slf| {
            slf.try_via_socket(
                on_connected.clone(),
                on_disconnected.clone(),
                verify_tls.clone(),
            )
        })
        .or_else(|slf| slf.try_via_network(on_connected, on_disconnected.clone(), verify_tls))
        .unwrap_or_else(|slf| slf.not_supported(on_disconnected))
    }
}

impl Adapter for LibvirtDynamicAdapter<VncAdapter> {
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
        verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> Box<dyn AdapterDisplay> {
        todo!()
    }
}
