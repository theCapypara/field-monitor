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
//! The backend part from qemu_display's perspective.
//! Handling device acquisition and release on the QEMU D-Bus side.

use crate::adapter::usbredir::qemu_dbus::portal_backend::QemuDbusAttacher;
use crate::adapter::usbredir::qemu_dbus::{FieldMonitorUsbRedirQemuDbus, FmDeviceKey};
use glib::{MainContext, SendWeakRef};
use log::{error, trace};
use rdw_qemu::qemu_display;
use rdw_qemu::usbredir::RusbSession;
use spice_gtk_usb_portal::devices::GenericOwnedUsbDevice;
use std::os::fd::{AsRawFd, BorrowedFd};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use zbus::export::async_trait;

/// Backend proxy struct for qemu_display.
#[derive(Debug, Clone)]
pub struct FmRedirectBackend {
    // IMPORTANT: As specified in `SendWeakRef`, this MUST be upgraded from the main thread,
    // -> spawn on default MainContext!
    pub implementation: SendWeakRef<FieldMonitorUsbRedirQemuDbus>,
}

// Session started with QEMU -> connect acquired PORTAL device
#[async_trait::async_trait]
impl qemu_display::UsbRedirBackend for FmRedirectBackend {
    type Device = FmDeviceKey;
    type Key = FmDeviceKey;
    type Session = FmRedirectSession;

    async fn start_session(
        &self,
        device: &Self::Device,
        stream: UnixStream,
    ) -> qemu_display::Result<Self::Session> {
        trace!("FmRedirectSession::start_session");
        let device_key = device.clone();
        let implementation = self.implementation.clone();
        MainContext::default()
            .spawn_from_within(move || async move {
                let implementation_strong = implementation
                    .upgrade()
                    .ok_or_else(|| qemu_display::Error::Failed("backend gone".to_string()))?;
                let device = implementation_strong
                    .move_acquired_device(&device_key)
                    .ok_or_else(|| {
                        qemu_display::Error::Failed("device not acquired".to_string())
                    })?;
                FmRedirectSession::new(implementation, device_key, device, stream)
            })
            .await
            .map_err(|_| qemu_display::Error::Failed("backend session join failed".to_string()))
            .flatten()
            .inspect_err(|err| error!("backend start session failed: {}", err))
    }
}

#[derive(Debug)]
pub struct FmRedirectSession {
    /// Pumps usbredir data between the portal device and the QEMU socket.
    /// If dropped, the device is effectively detached from QEMU.
    _session: RusbSession,
    implementation: SendWeakRef<FieldMonitorUsbRedirQemuDbus>,
    key: FmDeviceKey,
    /// We only hold the device alive because we hand libusb a duplicate of it's fd.
    /// Don't use it directly in Drop, see notes in Drop.
    _device: Arc<GenericOwnedUsbDevice<QemuDbusAttacher>>,
}

impl FmRedirectSession {
    fn new(
        implementation: SendWeakRef<FieldMonitorUsbRedirQemuDbus>,
        key: FmDeviceKey,
        device: Arc<GenericOwnedUsbDevice<QemuDbusAttacher>>,
        stream: UnixStream,
    ) -> qemu_display::Result<Self> {
        // libusb does not take ownership of the fd it wraps, and the portal fd is owned
        // by `device`, which must be dropped on the main thread. Hand the session its
        // own duplicate instead.
        // Safety: the fd is valid, as it is owned by `device`, which we hold alive.
        let device_fd = unsafe { BorrowedFd::borrow_raw(device.as_raw_fd()) }
            .try_clone_to_owned()
            .map_err(|err| {
                qemu_display::Error::Failed(format!("failed to duplicate the device fd: {err}"))
            })?;

        // If the QEMU side of the stream dies (e.g. the VM went away), drop the session
        // from the channel manager, which cleans everything else up (see `Drop`).
        let on_disconnected = {
            let implementation = implementation.clone();
            let key = key.clone();
            Box::new(move || {
                MainContext::default().spawn_from_within(move || async move {
                    trace!("FmRedirectSession::disconnect");
                    if let Some(implementation) = implementation.upgrade() {
                        implementation.handle_session_disconnected(&key).await;
                    }
                });
            })
        };

        let session = RusbSession::from_fd(device_fd, stream, Some(on_disconnected))?;

        Ok(Self {
            _session: session,
            _device: device,
            implementation,
            key,
        })
    }
}

// QEMU session closed -> run PORTAL detach & update the device state in the store
impl Drop for FmRedirectSession {
    fn drop(&mut self) {
        trace!("FmRedirectSession::drop");
        let implementation = self.implementation.clone();
        let key = self.key.clone();
        // We go through the main implementation struct (=through the store) to detach the device,
        // instead of detaching `self.device` directly. This is important to make sure the UI
        // state is also properly updated, plus this ends up calling `device.detach()`.
        MainContext::default().spawn_from_within(move || async move {
            if let Some(implementation) = implementation.upgrade() {
                implementation.detach_disconnected_device_by_key(&key).await;
            }
        });
    }
}
