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

use futures::StreamExt;
mod device;
mod error;
pub mod spice;

pub use self::device::FieldMonitorUsbDevice;
pub use self::error::*;

use futures::future::LocalBoxFuture;
use glib::subclass::prelude::*;
use gtk::gio;
use gtk::gio::prelude::*;
use log::{debug, error, trace};
use std::cell::{Cell, OnceCell};

mod imp {
    use super::*;

    #[repr(C)]
    pub struct FieldMonitorUsbRedirAdapterClass {
        pub parent_class: glib::gobject_ffi::GObjectClass,

        pub attach_device: for<'a> fn(
            &'a super::FieldMonitorUsbRedirAdapter,
            &'a FieldMonitorUsbDevice,
            Option<&'a gtk::Window>,
        ) -> LocalBoxFuture<'a, UsbRedirResult<()>>,

        pub detach_device: for<'a> fn(
            &'a super::FieldMonitorUsbRedirAdapter,
            &'a FieldMonitorUsbDevice,
        ) -> LocalBoxFuture<'a, UsbRedirResult<()>>,
    }

    unsafe impl ClassStruct for FieldMonitorUsbRedirAdapterClass {
        type Type = FieldMonitorUsbRedirAdapter;
    }

    #[derive(Default, Debug, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorUsbRedirAdapter)]
    pub struct FieldMonitorUsbRedirAdapter {
        /// The maximum count of USB devices supported to be redirected.
        /// If this number is negative, an "infinite" amount of devices is
        /// supported and `free-channels` is always 0 (irrelevant).
        ///
        /// Not intended to be set, except from the implementation.
        #[property(get, set)]
        pub max_channels: Cell<i32>,

        /// Number of USB devices that can still be attached to the connection.
        ///
        /// Not intended to be set, except from the implementation.
        #[property(get, set)]
        pub free_channels: Cell<u32>,

        /// Devices available for redirection.
        /// Item type: [`FieldMonitorUsbDevice`].
        #[property(get, construct_only)]
        pub store: OnceCell<gio::ListStore>,

        pub(self) disposed: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorUsbRedirAdapter {
        const NAME: &'static str = "FieldMonitorUsbRedirAdapter";
        const ABSTRACT: bool = true;
        type Type = super::FieldMonitorUsbRedirAdapter;
        type Class = FieldMonitorUsbRedirAdapterClass;
        type ParentType = glib::Object;

        fn class_init(klass: &mut Self::Class) {
            klass.attach_device = |_, _, _| unimplemented!();
            klass.detach_device = |_, _| unimplemented!();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorUsbRedirAdapter {
        fn dispose(&self) {
            // we need a flag since cloning the object below to pass it into the async callback
            // will up the ref count, running dispose again.
            if !self.disposed.get() {
                trace!("Dispose FieldMonitorUsbRedirAdapter @ {self:?}");
                self.disposed.set(true);

                let obj = self.obj().clone();
                glib::spawn_future_local(async move {
                    for (dev, res) in obj.detach_all().await {
                        if let Err(err) = res {
                            error!("failed detaching {dev} during dispose: {err}");
                        }
                    }
                });
            }
        }
    }

    impl FieldMonitorUsbRedirAdapter {
        #[allow(dead_code)]
        pub(super) fn assert_in_store(&self, device: &FieldMonitorUsbDevice) {
            let store_brw = self.store.get();
            let store = store_brw.unwrap();
            assert!(
                store
                    .iter::<FieldMonitorUsbDevice>()
                    .any(|e| { e.unwrap() == *device })
            );
        }
    }

    impl Drop for FieldMonitorUsbRedirAdapter {
        fn drop(&mut self) {
            trace!("Drop FieldMonitorUsbRedirAdapter @ {self:?}");
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorUsbRedirAdapter(ObjectSubclass<imp::FieldMonitorUsbRedirAdapter>);
}

// -- instance methods
#[allow(async_fn_in_trait)]
pub trait FieldMonitorUsbRedirAdapterExt: 'static {
    /// Try to attach `device` to the connection. `device` must be from
    /// `self.store()`. Attaching the device may fail.
    ///
    /// The `current_window` is used to show permission dialogs of portals, if applicable.
    async fn attach_device(
        &self,
        device: &FieldMonitorUsbDevice,
        current_window: Option<&impl IsA<gtk::Window>>,
    ) -> UsbRedirResult<()>;

    /// Try to detach `device` from the connection. `device` must be from
    /// `self.store()`.
    /// Detaching the device may fail, in that case an error is returned.
    /// It if is not currently attached, this will do nothing.
    async fn detach_device(&self, device: &FieldMonitorUsbDevice) -> UsbRedirResult<()>;

    /// Detach all attached devices.
    #[must_use]
    async fn detach_all(&self) -> Vec<(FieldMonitorUsbDevice, UsbRedirResult<()>)>;
}

impl<O: IsA<FieldMonitorUsbRedirAdapter>> FieldMonitorUsbRedirAdapterExt for O {
    async fn attach_device(
        &self,
        device: &FieldMonitorUsbDevice,
        current_window: Option<&impl IsA<gtk::Window>>,
    ) -> UsbRedirResult<()> {
        // Safety: safe because IsA<FieldMonitorUsbRedirAdapter>
        let self_: &FieldMonitorUsbRedirAdapter = unsafe { self.unsafe_cast_ref::<_>() };

        if !device.attachable() {
            return Err(UsbRedirError::device_not_attachable());
        }

        // Check if in self.store()
        #[cfg(debug_assertions)]
        {
            let imp = self_.imp();
            imp.assert_in_store(device);
        }

        let klass = self_.class().as_ref();
        debug!("attaching {device}");
        let res =
            (klass.attach_device)(self_, device, current_window.map(|w| w.upcast_ref())).await;
        debug!("attaching {device} result: {res:?}");
        res
    }

    async fn detach_device(&self, device: &FieldMonitorUsbDevice) -> UsbRedirResult<()> {
        // Safety: safe because IsA<FieldMonitorUsbRedirAdapter>
        let self_: &FieldMonitorUsbRedirAdapter = unsafe { self.unsafe_cast_ref::<_>() };

        // Check if in self.store()
        #[cfg(debug_assertions)]
        {
            let imp = self_.imp();
            imp.assert_in_store(device);
        }

        if !device.attached() {
            return Ok(());
        }

        let klass = self_.class().as_ref();
        debug!("detaching {device}");
        let res = (klass.detach_device)(self_, device).await;
        debug!("detaching {device} result: {res:?}");
        res
    }

    async fn detach_all(&self) -> Vec<(FieldMonitorUsbDevice, UsbRedirResult<()>)> {
        // Safety: safe because IsA<FieldMonitorUsbRedirAdapter>
        let self_: &FieldMonitorUsbRedirAdapter = unsafe { self.unsafe_cast_ref::<_>() };

        let klass = self_.class().as_ref();
        let detach_device = &klass.detach_device;

        let imp = self_.imp();
        let store = imp.store.get();
        if let Some(store) = store.as_ref() {
            let devices = store
                .iter::<FieldMonitorUsbDevice>()
                .map(|r| r.unwrap())
                .collect::<Vec<_>>();
            futures::stream::iter(devices)
                .filter_map(|d| async move {
                    if d.attached() {
                        let result = detach_device(self_, &d).await;
                        Some((d, result))
                    } else {
                        None
                    }
                })
                .collect()
                .await
        } else {
            vec![]
        }
    }
}

// -- virtual methods
pub trait FieldMonitorUsbRedirAdapterImpl:
    ObjectImpl + ObjectSubclass<Type: IsA<FieldMonitorUsbRedirAdapter>>
{
    /// Try to attach `device` to the connection. Attaching the device may fail.
    fn attach_device<'a>(
        &'a self,
        _device: &'a FieldMonitorUsbDevice,
        _current_window: Option<&'a gtk::Window>,
    ) -> LocalBoxFuture<'a, UsbRedirResult<()>> {
        unimplemented!()
    }

    /// Try to detach `device` from the connection. `device` is currently attached.
    /// Detaching the device may fail, in that case an error is returned.
    fn detach_device<'a>(
        &'a self,
        _device: &'a FieldMonitorUsbDevice,
    ) -> LocalBoxFuture<'a, UsbRedirResult<()>> {
        unimplemented!()
    }
}

pub trait FieldMonitorUsbRedirAdapterImplExt: FieldMonitorUsbRedirAdapterImpl {}

impl<T: FieldMonitorUsbRedirAdapterImpl> FieldMonitorUsbRedirAdapterImplExt for T {}

unsafe impl<T: FieldMonitorUsbRedirAdapterImpl> IsSubclassable<T> for FieldMonitorUsbRedirAdapter {
    fn class_init(class: &mut glib::Class<Self>) {
        fn attach_device_trampoline<'a, T: FieldMonitorUsbRedirAdapterImpl>(
            obj: &'a FieldMonitorUsbRedirAdapter,
            device: &'a FieldMonitorUsbDevice,
            current_window: Option<&'a gtk::Window>,
        ) -> LocalBoxFuture<'a, UsbRedirResult<()>> {
            // safety: safe because we know this IsA<FieldMonitorUsbRedirAdapter>
            let imp = unsafe { obj.unsafe_cast_ref::<T::Type>() }.imp();
            FieldMonitorUsbRedirAdapterImpl::attach_device(imp, device, current_window)
        }
        fn detach_device_trampoline<'a, T: FieldMonitorUsbRedirAdapterImpl>(
            obj: &'a FieldMonitorUsbRedirAdapter,
            device: &'a FieldMonitorUsbDevice,
        ) -> LocalBoxFuture<'a, UsbRedirResult<()>> {
            // safety: safe because we know this IsA<FieldMonitorUsbRedirAdapter>
            let imp = unsafe { obj.unsafe_cast_ref::<T::Type>() }.imp();
            FieldMonitorUsbRedirAdapterImpl::detach_device(imp, device)
        }

        Self::parent_class_init::<T>(class);
        let klass = class.as_mut();
        klass.attach_device = attach_device_trampoline::<T>;
        klass.detach_device = detach_device_trampoline::<T>;
    }
}
