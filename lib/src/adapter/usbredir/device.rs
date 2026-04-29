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

use glib;
use glib::prelude::*;
use glib::subclass::prelude::*;
use std::cell::{Cell, OnceCell, RefCell};
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorUsbDevice)]
    #[derive(Debug)]
    pub struct FieldMonitorUsbDevice {
        /// Inner device information struct. Can be null if not relevant for the implementation.
        pub(super) inner: OnceCell<glib::Object>,
        /// Handle to a currently attached device. If not null,
        /// the device is considered attached.
        pub(super) attached_handle: RefCell<Option<glib::Object>>,
        #[property(get, set)]
        /// Device model.
        pub(super) model: RefCell<String>,
        #[property(get, set)]
        /// Device vendor.
        pub(super) vendor: RefCell<String>,
        #[property(get, set)]
        /// Can be attached. This is usually set to `false` if the user
        /// does not have permissions to share this device.
        pub(super) attachable: Cell<bool>,
        #[property(get, set)]
        /// Whether this device is a USB hub or similar (billboard, etc.) device.
        pub(super) hub: Cell<bool>,
        #[property(get = Self::is_attached, explicit_notify, default = false)]
        /// Whether this device is currently attached.
        pub(super) attached: PhantomData<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorUsbDevice {
        const NAME: &'static str = "FieldMonitorUsbDevice";
        type Type = super::FieldMonitorUsbDevice;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorUsbDevice {}

    impl FieldMonitorUsbDevice {
        fn is_attached(&self) -> bool {
            self.attached_handle.borrow().is_some()
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorUsbDevice(ObjectSubclass<imp::FieldMonitorUsbDevice>);
}

impl Display for FieldMonitorUsbDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[USB device '{}' - '{}']", self.vendor(), self.model())
    }
}

impl FieldMonitorUsbDevice {
    pub(crate) fn new(
        inner: Option<&impl IsA<glib::Object>>,
        model: &str,
        vendor: &str,
        attachable: bool,
        hub: bool,
    ) -> Self {
        let slf: Self = glib::Object::builder()
            .property("model", model)
            .property("vendor", vendor)
            .property("attachable", attachable)
            .property("hub", hub)
            .build();
        if let Some(inner) = inner {
            slf.imp().inner.set(inner.upcast_ref().clone()).unwrap();
        }
        slf
    }

    pub(crate) fn inner(&self) -> Option<&glib::Object> {
        self.imp().inner.get()
    }

    pub(crate) fn attached_handle(&self) -> impl Deref<Target = Option<glib::Object>> {
        self.imp().attached_handle.borrow()
    }

    pub(crate) fn set_attached_handle(&self, v: Option<glib::Object>) {
        {
            *self.imp().attached_handle.borrow_mut() = v;
        }
        self.notify_attached();
    }

    pub(crate) fn update(
        &self,
        model: Option<&str>,
        vendor: Option<&str>,
        attachable: Option<bool>,
        hub: Option<bool>,
    ) {
        let imp = self.imp();
        if let Some(model) = model
            && model != self.model()
        {
            *imp.model.borrow_mut() = model.to_string();
            self.notify_model();
        }
        if let Some(vendor) = vendor
            && vendor != self.vendor()
        {
            *imp.vendor.borrow_mut() = vendor.to_string();
            self.notify_vendor();
        }
        if let Some(attachable) = attachable
            && attachable != self.attachable()
        {
            imp.attachable.set(attachable);
            self.notify_attachable();
        }
        if let Some(hub) = hub
            && hub != self.hub()
        {
            imp.hub.set(hub);
            self.notify_hub();
        }
    }
}
