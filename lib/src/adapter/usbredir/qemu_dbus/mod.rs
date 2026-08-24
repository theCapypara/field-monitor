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
mod portal_backend;
mod qemu_display_backend;

use crate::adapter::usbredir::FmUsbRedirError;
use crate::adapter::usbredir::portal::FieldMonitorUsbRedirPortalDevices;
use crate::adapter::usbredir::qemu_dbus::portal_backend::{QemuAttachedDevice, QemuDbusAttacher};
use crate::adapter::usbredir::qemu_dbus::qemu_display_backend::FmRedirectBackend;
use crate::adapter::usbredir::{
    FieldMonitorUsbDevice, FieldMonitorUsbRedirAdapter, FieldMonitorUsbRedirAdapterExt,
    FieldMonitorUsbRedirAdapterImpl, FmUsbRedirResult,
};
use futures::StreamExt;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use glib::WeakRef;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use log::{debug, trace, warn};
use rdw_qemu::qemu_display;
use spice_gtk_usb_portal::DeviceID;
use spice_gtk_usb_portal::devices::GenericOwnedUsbDevice;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// A key to get a device from the FieldMonitorUsbRedirQemuDbus's store. We can't Send+Sync the
/// FieldMonitorUsbDevice objects, so we need to use some indirection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FmDeviceKey(DeviceID);

// needed for qemu_display::UsbRedirBackend::Key
impl From<&FmDeviceKey> for FmDeviceKey {
    fn from(value: &FmDeviceKey) -> Self {
        value.clone()
    }
}

mod imp {
    use super::*;

    #[derive(Default, Debug)]
    pub struct FieldMonitorUsbRedirQemuDbus {
        pub(super) devices: OnceCell<FieldMonitorUsbRedirPortalDevices>,
        // Currently acquired devices that haven't been connected to the QEMU-end yet.
        pub(super) acquired_devices:
            RefCell<HashMap<FmDeviceKey, Arc<GenericOwnedUsbDevice<QemuDbusAttacher>>>>,
        pub(super) inner: OnceCell<qemu_display::UsbRedir<FmRedirectBackend>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorUsbRedirQemuDbus {
        const NAME: &'static str = "FieldMonitorUsbRedirQemuDbus";
        type Type = super::FieldMonitorUsbRedirQemuDbus;
        type ParentType = FieldMonitorUsbRedirAdapter;
    }

    impl ObjectImpl for FieldMonitorUsbRedirQemuDbus {}

    impl FieldMonitorUsbRedirAdapterImpl for FieldMonitorUsbRedirQemuDbus {
        fn attach_device<'a>(
            &'a self,
            device: &'a FieldMonitorUsbDevice,
            current_window: Option<&'a gtk::Window>,
        ) -> LocalBoxFuture<'a, FmUsbRedirResult<()>> {
            Box::pin(glib::clone!(
                #[strong(rename_to=slf)]
                self,
                async move {
                    trace!(
                        "<FieldMonitorUsbRedirQemuDbus as FieldMonitorUsbRedirAdapterImpl>::attach_device"
                    );
                    let redir = slf.inner.get().unwrap();
                    let devices = slf.devices.get().unwrap();
                    let device_description = device.description().unwrap();
                    let key = FmDeviceKey(device_description.id().clone());

                    if device.attached() || redir.is_device_connected(&key).await {
                        return Err(FmUsbRedirError::device_already_attached());
                    }

                    let attacher = QemuDbusAttacher {
                        redir: redir.clone(),
                        key: key.clone(),
                        attached: false,
                    };

                    // 1. Acquire device
                    let owned = Arc::new(
                        devices
                            .acquire_device(attacher, current_window, device_description.id(), true)
                            .await?,
                    );
                    slf.acquired_devices
                        .borrow_mut()
                        .insert(key.clone(), owned.clone());

                    // 2. Attach device
                    if let Err(err) = redir
                        .set_device_state(&key, true)
                        .await
                        .map_err(|e| FmUsbRedirError(format!("{}: {}", failed_text(), e)))
                    {
                        // on error, clean up / disconnect device again from the portal end
                        slf.acquired_devices.borrow_mut().remove(&key);
                        return Err(err);
                    }

                    // 3. Mark the portal device as attached, so all portal-side teardown
                    //    paths know to detach it from QEMU. The data path itself was
                    //    already set up by the session started in step 2.
                    if let Err(err) = owned.attach(&()).await {
                        // can only be `AlreadyAttached`, which would be a bug, but is
                        // harmless at this point
                        warn!("failed to mark USB device as attached: {err}");
                    }
                    device.set_attached_device(Box::new(QemuAttachedDevice(owned)));

                    // The session may already be gone again (e.g. the VM went away while
                    // we were attaching). Roll back in that case.
                    if !redir.is_device_connected(&key).await {
                        let _ = device.detach().await;
                        return Err(FmUsbRedirError(failed_text()));
                    }
                    Ok(())
                }
            ))
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorUsbRedirQemuDbus(ObjectSubclass<imp::FieldMonitorUsbRedirQemuDbus>) @extends FieldMonitorUsbRedirAdapter;
}

impl FieldMonitorUsbRedirQemuDbus {
    pub(crate) async fn new(chardevs: Vec<qemu_display::Chardev>) -> FmUsbRedirResult<Self> {
        trace!("FieldMonitorUsbRedirQemuDbus::new");
        let devices = FieldMonitorUsbRedirPortalDevices::new().await?;

        let slf: Self = glib::Object::builder()
            .property("store", devices.store())
            .property("max-channels", chardevs.len() as i32)
            .build();

        devices.connect_device_removed_to_impl(&slf);

        let inner = qemu_display::UsbRedir::new(
            chardevs,
            FmRedirectBackend {
                implementation: slf.clone().downgrade().into(),
            },
        );
        glib::spawn_future_local(Self::update_free_channels_loop(
            inner.clone(),
            slf.clone().downgrade(),
        ));

        let imp = slf.imp();
        imp.devices.set(devices).unwrap();
        imp.inner.set(inner).unwrap();

        Ok(slf)
    }

    fn move_acquired_device(
        &self,
        key: &FmDeviceKey,
    ) -> Option<Arc<GenericOwnedUsbDevice<QemuDbusAttacher>>> {
        trace!("FieldMonitorUsbRedirQemuDbus::move_acquired_device");
        let mut acquired_devices = self.imp().acquired_devices.borrow_mut();
        acquired_devices.remove(key)
    }

    /// Called when the QEMU-side stream of a session died (e.g. the VM went away).
    /// Drops the session from the channel manager, freeing the channel again;
    /// no-op if the session is already gone (regular detach).
    async fn handle_session_disconnected(&self, key: &FmDeviceKey) {
        trace!("FieldMonitorUsbRedirQemuDbus::handle_session_disconnected");
        debug!("QEMU usbredir session for {key:?} disconnected");
        if let Some(redir) = self.imp().inner.get()
            && let Err(err) = redir.set_device_state(key, false).await
        {
            warn!("failed to clean up disconnected USB device session: {err}");
        }
    }

    /// Detach a device, unless a session for it is active.
    async fn detach_disconnected_device_by_key(&self, key: &FmDeviceKey) {
        trace!("FieldMonitorUsbRedirQemuDbus::detach_disconnected_device_by_key");
        let Some(redir) = self.imp().inner.get() else {
            return;
        };
        if redir.is_device_connected(key).await {
            return;
        }
        let Some(devices) = self.imp().devices.get() else {
            return;
        };
        // TODO: This is not the nicest looking code... (and also not the fastest)
        let device = devices.iter_store().find_map(|(_, entry)| {
            entry.and_then(|(desc, device)| (desc.id() == &key.0).then_some(device))
        });
        if let Some(device) = device
            && device.attached()
            && let Err(err) = self.detach_device(&device).await
        {
            warn!("failed to update USB device state after session end: {err}");
        }
    }

    async fn update_free_channels_loop(
        usbredir: qemu_display::UsbRedir<FmRedirectBackend>,
        slf: WeakRef<Self>,
    ) {
        trace!("FieldMonitorUsbRedirQemuDbus::update_free_channels_loop");
        {
            let Some(slf) = slf.upgrade() else {
                return;
            };
            slf.set_property("free-channels", usbredir.n_free_channels().await as u32);
        }
        let mut n = usbredir.receive_n_free_channels().await;
        while let Some(n) = n.next().await {
            trace!("FieldMonitorUsbRedirQemuDbus::update_free_channels_loop (new n={n})");
            let Some(slf) = slf.upgrade() else {
                break;
            };
            let supr = slf.upcast_ref::<FieldMonitorUsbRedirAdapter>();
            let cur_val = supr.free_channels();
            if cur_val != (n as u32) {
                supr.set_free_channels(n as u32);
            }
        }
    }
}

fn failed_text() -> String {
    gettext("Failed to attach the USB device")
}
