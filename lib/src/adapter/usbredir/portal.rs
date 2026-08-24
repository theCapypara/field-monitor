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
//! Common USB portal implementation. Even though this uses `spice-gtk-usb-portal`,
//! this can also be used with non-SPICE implementations.

use crate::adapter::usbredir::{
    FieldMonitorUsbDevice, FieldMonitorUsbRedirAdapter, FieldMonitorUsbRedirAdapterExt,
    FmUsbRedirError, FmUsbRedirResult,
};
use crate::i18n::gettext_f;
use glib::subclass::Signal;
use glib::subclass::prelude::*;
use gtk::gio;
use gtk::prelude::*;
use log::warn;
use spice_gtk_usb_portal::devices::{
    DeviceDescription, DeviceError, DeviceResult, Devices, GenericOwnedUsbDevice, OwnedUsbDevice,
    PortalUsbredirAttacher,
};
use spice_gtk_usb_portal::{DeviceID, WindowIdentifier};
use std::cell::OnceCell;
use std::sync::OnceLock;

mod imp {
    use super::*;

    #[derive(Default, Debug, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorUsbRedirPortalDevices)]
    pub struct FieldMonitorUsbRedirPortalDevices {
        #[property(get, construct_only)]
        pub(super) store: OnceCell<gio::ListStore>,
        pub(super) devices: OnceCell<Devices>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorUsbRedirPortalDevices {
        const NAME: &'static str = "FieldMonitorUsbRedirPortalDevices";
        type Type = super::FieldMonitorUsbRedirPortalDevices;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorUsbRedirPortalDevices {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    // A device has been removed.
                    Signal::builder("device-removed")
                        .param_types([FieldMonitorUsbDevice::static_type()])
                        .build(),
                ]
            })
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorUsbRedirPortalDevices(ObjectSubclass<imp::FieldMonitorUsbRedirPortalDevices>);
}

impl FieldMonitorUsbRedirPortalDevices {
    pub async fn new() -> FmUsbRedirResult<Self> {
        let store = gio::ListStore::builder()
            .item_type(FieldMonitorUsbDevice::static_type())
            .build();

        let slf: Self = glib::Object::builder().property("store", &store).build();
        let imp = slf.imp();

        let devices = Devices::new().await?;

        slf.setup_store(&devices).await;

        imp.devices.set(devices).unwrap();
        Ok(slf)
    }

    pub async fn acquire_device_spice(
        &self,
        parent_window: Option<&gtk::Window>,
        device_id: &DeviceID,
        writable: bool,
    ) -> DeviceResult<OwnedUsbDevice> {
        let parent_window = match parent_window {
            Some(w) => WindowIdentifier::from_native(w).await,
            None => None,
        };
        self.imp()
            .devices
            .get()
            .unwrap()
            .acquire_device(parent_window.as_ref(), device_id, writable)
            .await
    }

    pub async fn acquire_device<T: PortalUsbredirAttacher>(
        &self,
        attacher: T,
        parent_window: Option<&gtk::Window>,
        device_id: &DeviceID,
        writable: bool,
    ) -> DeviceResult<GenericOwnedUsbDevice<T>> {
        let parent_window = match parent_window {
            Some(w) => WindowIdentifier::from_native(w).await,
            None => None,
        };
        self.imp()
            .devices
            .get()
            .unwrap()
            .acquire_device_with_attacher(attacher, parent_window.as_ref(), device_id, writable)
            .await
    }

    pub fn connect_device_removed_to_impl(
        &self,
        implementation: &impl IsA<FieldMonitorUsbRedirAdapter>,
    ) {
        let implementation = implementation.clone().upcast();
        self.connect_closure(
            "device-removed",
            false,
            glib::closure_local!(
                #[watch]
                implementation,
                move |_: Self, device: FieldMonitorUsbDevice| {
                    glib::spawn_future_local(glib::clone!(
                        #[weak]
                        implementation,
                        async move {
                            implementation.detach_device(&device).await.ok();
                        }
                    ));
                }
            ),
        );
    }

    pub fn iter_store(
        &self,
    ) -> impl Iterator<Item = (u32, Option<(DeviceDescription, FieldMonitorUsbDevice)>)> {
        let store = self.store();
        (0..store.n_items()).map(move |i| (i, store_get_device_w_description(&store, i)))
    }

    // TODO: we could cache a mapping of device id -> index
    fn contains_id(&self, id: &DeviceID) -> bool {
        for (_, opt) in self.iter_store() {
            if let Some((desc, _)) = opt
                && desc.id() == id
            {
                return true;
            }
        }
        false
    }

    async fn setup_store(&self, devices: &Devices) {
        devices.connect_closure(
            "device-added",
            false,
            glib::closure_local!(
                #[weak(rename_to=slf)]
                self,
                move |_: Devices, desc: DeviceDescription| {
                    if !slf.contains_id(desc.id()) {
                        slf.store().append(&new_device(&desc));
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
                move |_: Devices, desc: DeviceDescription| {
                    for (i, opt) in slf.iter_store() {
                        if let Some((device_desc, device)) = opt
                            && device_desc.id() == desc.id()
                        {
                            slf.store().remove(i);
                            slf.emit_by_name::<()>("device-removed", &[&device]);
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
                #[weak(rename_to=slf)]
                self,
                move |_: Devices, desc: DeviceDescription| {
                    for (_, opt) in slf.iter_store() {
                        if let Some((device_desc, device)) = opt
                            && device_desc.id() == desc.id()
                        {
                            update_device(&device, &desc);
                            return;
                        }
                    }
                    // Device wasn't in the store yet — treat as an add.
                    slf.store().append(&new_device(&desc));
                }
            ),
        );

        glib::spawn_future_local(glib::clone!(
            #[strong]
            devices,
            #[weak(rename_to=slf)]
            self,
            async move {
                match devices.enumerate_devices().await {
                    Ok(list) => {
                        for desc in list {
                            if !slf.contains_id(desc.id()) {
                                slf.store().append(&new_device(&desc));
                            }
                        }
                    }
                    Err(e) => warn!("enumerate_devices failed: {e}"),
                }
            }
        ));
    }
}

impl From<DeviceError> for FmUsbRedirError {
    fn from(value: DeviceError) -> Self {
        Self(match value {
            DeviceError::Portal(err) => gettext_f(
                "Failed to communicate with the system portal: {details}",
                &[("details", &err.to_string())],
            ),
            DeviceError::Usb(err) => gettext_f(
                "USB portal request failed: {details}",
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

// TODO: we could cache a mapping of index -> (device description, device)
fn store_get_device_w_description(
    store: &gio::ListStore,
    i: u32,
) -> Option<(DeviceDescription, FieldMonitorUsbDevice)> {
    let usb_device = store.item(i).and_downcast::<FieldMonitorUsbDevice>()?;
    let inner = usb_device.description()?;
    let desc = inner.clone();
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
