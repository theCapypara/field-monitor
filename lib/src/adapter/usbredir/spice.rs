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
use crate::i18n::gettext_f;
use gettextrs::gettext;
use glib::subclass::prelude::*;
use log::warn;
use spice_gtk_usb_portal::devices::{DeviceDescription, DeviceError, Devices, OwnedUsbDevice};
use spice_gtk_usb_portal::{DeviceID, Usbredir, UsbredirError, WindowIdentifier};
use std::cell::OnceCell;

mod imp {
    use super::*;

    #[derive(Default, Debug)]
    pub struct FieldMonitorUsbRedirSpice {
        pub(super) devices: OnceCell<Devices>,
        pub(super) inner: OnceCell<Usbredir>,
        // self.store (FieldMonitorUsbDevice) fields:
        // -> inner: DeviceDescription
        // -> attached_handle: OwnedUsbDevice
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
        ) -> LocalBoxFuture<'a, UsbRedirResult<()>> {
            Box::pin(glib::clone!(
                #[strong(rename_to=slf)]
                self,
                #[strong]
                device,
                #[strong]
                current_window,
                async move {
                    let inner = slf.inner.get().unwrap();
                    let devices = slf.devices.get().unwrap();
                    let device_description_obj = device.inner().unwrap();
                    let device_description: &DeviceDescription =
                        device_description_obj.downcast_ref().unwrap();

                    // 1. Request device
                    let parent_id = match current_window {
                        Some(w) => WindowIdentifier::from_native(w).await,
                        None => None,
                    };
                    let owned = devices
                        .acquire_device(parent_id.as_ref(), device_description.id(), true)
                        .await?;

                    // 2. Attach device
                    let res = inner.attach(&owned).await;
                    if res.is_ok() {
                        device.set_attached_handle(Some(owned.upcast()));
                    }
                    res.map_err(Into::into)
                }
            ))
        }

        fn detach_device<'a>(
            &'a self,
            device: &'a FieldMonitorUsbDevice,
        ) -> LocalBoxFuture<'a, UsbRedirResult<()>> {
            Box::pin(glib::clone!(
                #[strong]
                device,
                async move {
                    let device_inner_ref: OwnedUsbDevice = {
                        let device_handle_brw = device.attached_handle();
                        let device_handle_ref = device_handle_brw.as_ref().unwrap();

                        device_handle_ref.clone().downcast().unwrap()
                    };
                    device_inner_ref.detach_from_spice().await;
                    device.set_attached_handle(None);
                    Ok(())
                }
            ))
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorUsbRedirSpice(ObjectSubclass<imp::FieldMonitorUsbRedirSpice>) @extends FieldMonitorUsbRedirAdapter;
}

impl FieldMonitorUsbRedirSpice {
    pub(crate) async fn new(session: &rdw_spice::spice::Session) -> UsbRedirResult<Self> {
        let store = gio::ListStore::builder()
            .item_type(FieldMonitorUsbDevice::static_type())
            .build();

        let slf: Self = glib::Object::builder().property("store", &store).build();
        let imp = slf.imp();

        let devices = Devices::new().await?;
        let inner = Usbredir::new(session)?;

        inner
            .bind_property("free-channels", &slf, "free-channels")
            .sync_create()
            .build();
        inner
            .bind_property("max-channels", &slf, "max-channels")
            .sync_create()
            .build();

        slf.setup_store(&store, &devices).await;

        imp.devices.set(devices).unwrap();
        imp.inner.set(inner).unwrap();
        Ok(slf)
    }

    async fn setup_store(&self, store: &gio::ListStore, devices: &Devices) {
        devices.connect_closure(
            "device-added",
            false,
            glib::closure_local!(
                #[weak]
                store,
                move |_: Devices, desc: DeviceDescription| {
                    if !contains_id(&store, desc.id()) {
                        store.append(&new_device(&desc));
                    }
                }
            ),
        );

        devices.connect_closure(
            "device-removed",
            false,
            glib::closure_local!(
                #[weak(rename_to=slf)]
                self,
                #[weak]
                store,
                move |_: Devices, desc: DeviceDescription| {
                    for i in 0..store.n_items() {
                        let Some((device_desc, device)) = store_get_device_w_description(&store, i)
                        else {
                            continue;
                        };
                        if device_desc.id() == desc.id() {
                            store.remove(i);

                            // Disconnect device if attached
                            glib::spawn_future_local(glib::clone!(
                                #[strong]
                                slf,
                                #[strong]
                                device,
                                async move { slf.imp().detach_device(&device).await.ok() }
                            ));
                            break;
                        }
                    }
                }
            ),
        );

        devices.connect_closure(
            "device-changed",
            false,
            glib::closure_local!(
                #[weak]
                store,
                move |_: Devices, desc: DeviceDescription| {
                    for i in 0..store.n_items() {
                        let Some((device_desc, device)) = store_get_device_w_description(&store, i)
                        else {
                            continue;
                        };
                        if device_desc.id() == desc.id() {
                            update_device(&device, &desc);
                            return;
                        }
                    }
                    // Device wasn't in the store yet — treat as an add.
                    store.append(&new_device(&desc));
                }
            ),
        );

        glib::spawn_future_local(glib::clone!(
            #[strong]
            devices,
            #[weak]
            store,
            async move {
                match devices.enumerate_devices().await {
                    Ok(list) => {
                        for desc in list {
                            if !contains_id(&store, desc.id()) {
                                store.append(&new_device(&desc));
                            }
                        }
                    }
                    Err(e) => warn!("enumerate_devices failed: {e}"),
                }
            }
        ));
    }
}

impl From<DeviceError> for UsbRedirError {
    fn from(value: DeviceError) -> Self {
        Self(match value {
            DeviceError::Portal(err) => gettext_f(
                "Failed to communicate with the system portal: {details}",
                &[("details", &err.to_string())],
            ),
            DeviceError::Usb(err) => gettext_f(
                "USB device communication failed: {details}",
                &[("details", &err.to_string())],
            ),
            DeviceError::Init(err) => gettext_f(
                "USB redirection initialization failed: {details}",
                &[("details", &err.to_string())],
            ),
            other => other.to_string(),
        })
    }
}

impl From<UsbredirError> for UsbRedirError {
    fn from(value: UsbredirError) -> Self {
        Self(match value {
            UsbredirError::Glib(err) => err.to_string(),
            // this is not a realistic error case, unless we hit a very bad bug, so we don't translate
            UsbredirError::NotConnected => "the connection was not connected".to_string(),
            UsbredirError::DeviceAttachFailed => gettext("Failed to attach the USB device"),
            other => other.to_string(),
        })
    }
}

// TODO: we could cache a mapping of device id -> index
fn contains_id(store: &gio::ListStore, id: &DeviceID) -> bool {
    for i in 0..store.n_items() {
        let Some((desc, _)) = store_get_device_w_description(store, i) else {
            continue;
        };
        if desc.id() == id {
            return true;
        }
    }
    false
}

// TODO: we could cache a mapping of index -> (device description, device)
fn store_get_device_w_description(
    store: &gio::ListStore,
    i: u32,
) -> Option<(DeviceDescription, FieldMonitorUsbDevice)> {
    let usb_device = store.item(i).and_downcast::<FieldMonitorUsbDevice>()?;
    let inner = usb_device.inner()?;
    let desc = inner.clone().downcast().ok()?;
    Some((desc, usb_device))
}

fn new_device(desc: &DeviceDescription) -> FieldMonitorUsbDevice {
    FieldMonitorUsbDevice::new(
        Some(desc),
        desc.model().unwrap_or_default().as_str(),
        desc.vendor().unwrap_or_default().as_str(),
        desc.readable() && desc.writable(),
        desc.is_likely_usb_hub(),
    )
}

fn update_device(device: &FieldMonitorUsbDevice, desc: &DeviceDescription) {
    device.update(
        desc.model().as_deref(),
        desc.vendor().as_deref(),
        Some(desc.readable() && desc.writable()),
        Some(desc.is_likely_usb_hub()),
    )
}
