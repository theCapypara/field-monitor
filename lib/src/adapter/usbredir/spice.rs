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
use super::*;
use crate::adapter::usbredir::portal::FieldMonitorUsbRedirPortalDevices;
use gettextrs::gettext;
use glib::subclass::prelude::*;
use spice_gtk_usb_portal::Usbredir;
use spice_gtk_usb_portal::devices::OwnedUsbDevice;
use std::cell::OnceCell;

impl FieldMonitorUsbRedirAttachedDevice for OwnedUsbDevice {
    fn detach(&self) -> LocalBoxFuture<'_, FmUsbRedirResult<()>> {
        Box::pin(async move {
            OwnedUsbDevice::detach_from_spice(self).await;
            Ok(())
        })
    }
}

mod imp {
    use super::*;

    #[derive(Default, Debug)]
    pub struct FieldMonitorUsbRedirSpice {
        pub(super) devices: OnceCell<FieldMonitorUsbRedirPortalDevices>,
        pub(super) inner: OnceCell<Usbredir>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorUsbRedirSpice {
        const NAME: &'static str = "FieldMonitorUsbRedirSpice";
        type Type = super::FieldMonitorUsbRedirSpice;
        type ParentType = FieldMonitorUsbRedirAdapter;
    }

    impl ObjectImpl for FieldMonitorUsbRedirSpice {}

    impl FieldMonitorUsbRedirAdapterImpl for FieldMonitorUsbRedirSpice {
        fn attach_device<'a>(
            &'a self,
            device: &'a FieldMonitorUsbDevice,
            current_window: Option<&'a gtk::Window>,
        ) -> LocalBoxFuture<'a, FmUsbRedirResult<()>> {
            Box::pin(glib::clone!(
                #[strong(rename_to=slf)]
                self,
                #[strong]
                device,
                #[strong]
                current_window,
                async move {
                    if device.attached() {
                        return Err(FmUsbRedirError::device_already_attached());
                    }
                    let inner = slf.inner.get().unwrap();
                    let devices = slf.devices.get().unwrap();
                    let device_description = device.description().unwrap();

                    // 1. Request device
                    let owned = devices
                        .acquire_device_spice(current_window, device_description.id(), true)
                        .await?;

                    // 2. Attach device
                    let res = inner.attach(&owned).await;
                    if res.is_ok() {
                        device.set_attached_device(Box::new(owned));
                    }
                    res.map_err(Into::into)
                }
            ))
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorUsbRedirSpice(ObjectSubclass<imp::FieldMonitorUsbRedirSpice>) @extends FieldMonitorUsbRedirAdapter;
}

impl FieldMonitorUsbRedirSpice {
    pub(crate) async fn new(session: &rdw_spice::spice::Session) -> FmUsbRedirResult<Self> {
        let devices = FieldMonitorUsbRedirPortalDevices::new().await?;

        let slf: Self = glib::Object::builder()
            .property("store", devices.store())
            .build();

        let inner = Usbredir::new(session)?;
        inner
            .bind_property("free-channels", &slf, "free-channels")
            .sync_create()
            .build();
        inner
            .bind_property("max-channels", &slf, "max-channels")
            .sync_create()
            .build();

        devices.connect_device_removed_to_impl(&slf);

        let imp = slf.imp();
        imp.devices.set(devices).unwrap();
        imp.inner.set(inner).unwrap();

        Ok(slf)
    }
}

impl From<spice_gtk_usb_portal::UsbredirError> for FmUsbRedirError {
    fn from(value: spice_gtk_usb_portal::UsbredirError) -> Self {
        Self(match value {
            spice_gtk_usb_portal::UsbredirError::Glib(err) => err.to_string(),
            // this is not a realistic error case, unless we hit a very bad bug, so we don't translate
            spice_gtk_usb_portal::UsbredirError::NotConnected => {
                "the connection was not connected".to_string()
            }
            spice_gtk_usb_portal::UsbredirError::DeviceAttachFailed => {
                gettext("Failed to attach the USB device")
            }
            other => other.to_string(),
        })
    }
}
