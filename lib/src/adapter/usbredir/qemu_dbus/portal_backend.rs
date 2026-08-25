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
//! The backend part from portal's perspective.
//! Handling device acquisition and release on the Portal side.

use crate::adapter::usbredir::qemu_dbus::FmDeviceKey;
#[allow(unused_imports)]
use crate::adapter::usbredir::qemu_dbus::qemu_display_backend::{
    FmRedirectBackend, FmRedirectSession,
};
use crate::adapter::usbredir::{FieldMonitorUsbRedirAttachedDevice, FmUsbRedirResult};
use futures::future::LocalBoxFuture;
use log::{trace, warn};
use rdw_qemu::qemu_display;
use spice_gtk_usb_portal::devices::{GenericOwnedUsbDevice, PortalFd, PortalUsbredirAttacher};
use spice_gtk_usb_portal::{UsbredirError, UsbredirResult};
use std::sync::Arc;

/// Handle stored in a [`FieldMonitorUsbDevice`] while it is attached. Detaching
/// asks qemu_display to disconnect the device.
#[derive(Debug)]
pub struct QemuAttachedDevice(pub Arc<GenericOwnedUsbDevice<QemuDbusAttacher>>);

// APP asked for detach -> detach at QEMU (via PORTAL trait)
impl FieldMonitorUsbRedirAttachedDevice for QemuAttachedDevice {
    fn detach(&self) -> LocalBoxFuture<'_, FmUsbRedirResult<()>> {
        Box::pin(async move {
            trace!("<QemuAttachedDevice as FieldMonitorUsbRedirAttachedDevice>::detach");
            self.0.detach().await;
            Ok(())
        })
    }
}

#[derive(Debug)]
pub struct QemuDbusAttacher {
    pub redir: qemu_display::UsbRedir<FmRedirectBackend>,
    pub key: FmDeviceKey,
    pub attached: bool,
}

// Have gotten device from PORTAL -> attach at QEMU
// (or have gotten detach request via PORTAL/APP -> detach at QEMU)
/// Attacher for Portal-devices to QEMU/D-Bus connections.
/// This actually doesn't do the actual attachment,
/// this is handled via [`FmRedirectBackend`] / [`FmRedirectSession`]
/// when the device state is set at the [`qemu_display`] end.
///
/// This struct effectively only keeps track of the current attachment state
/// and disconnects the device at the [`qemu_display`] end (by setting the
/// device state) on detach.
impl PortalUsbredirAttacher for QemuDbusAttacher {
    type AttachBackend = ();

    fn attach(
        &mut self,
        _device: Arc<PortalFd>,
        _backend: &Self::AttachBackend,
    ) -> LocalBoxFuture<'_, UsbredirResult<()>> {
        Box::pin(async move {
            trace!("<QemuDbusAttacher as PortalUsbredirAttacher>::start_session");
            if self.attached {
                return Err(UsbredirError::AlreadyAttached);
            }
            self.attached = true;
            Ok(())
        })
    }

    fn detach(&mut self) -> LocalBoxFuture<'_, ()> {
        Box::pin(async move {
            trace!("<QemuDbusAttacher as PortalUsbredirAttacher>::detach");
            self.attached = false;
            // If the session is already gone (e.g. it died and its drop cleared the
            // device state), qemu_display treats this as a no-op and still returns Ok.
            let _ = self
                .redir
                .set_device_state(&self.key, false)
                .await
                .map(drop)
                .inspect_err(|e| warn!("failed to detach the USB device: {}", e));
        })
    }

    fn is_attached(&self) -> bool {
        trace!("<QemuDbusAttacher as PortalUsbredirAttacher>::is_attached");
        self.attached
    }
}
