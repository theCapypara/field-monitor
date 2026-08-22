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
use glib::subclass::prelude::*;
use rdw_qemu::qemu_display::Chardev;

mod imp {
    use super::*;

    #[derive(Default, Debug)]
    pub struct FieldMonitorUsbRedirQemuDbus {
        // TODO
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
        ) -> LocalBoxFuture<'a, UsbRedirResult<()>> {
            Box::pin(glib::clone!(
                #[strong(rename_to=slf)]
                self,
                async move { todo!() }
            ))
        }

        fn detach_device<'a>(
            &'a self,
            device: &'a FieldMonitorUsbDevice,
        ) -> LocalBoxFuture<'a, UsbRedirResult<()>> {
            Box::pin(glib::clone!(
                #[strong]
                device,
                async move { todo!() }
            ))
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorUsbRedirQemuDbus(ObjectSubclass<imp::FieldMonitorUsbRedirQemuDbus>) @extends FieldMonitorUsbRedirAdapter;
}

impl FieldMonitorUsbRedirQemuDbus {
    pub(crate) async fn new(chardevs: Vec<Chardev>) -> UsbRedirResult<Self> {
        // let usbredir = UsbRedir::new(chardevs, usbredir::RusbBackend);
        // app_clone.set_usbredir(usbredir::Handler::new(usbredir));
        /*
        #[cfg(unix)]
        {
            let action_usb = gio::SimpleAction::new("usb", None);
            let app_clone = app.clone();
            action_usb.connect_activate(move |_, _| {
                let usbredir = app_clone.inner.usbredir.borrow();
                if let Some(usbredir) = usbredir.as_ref() {
                    let dialog = gtk::Window::new();
                    dialog.set_transient_for(app_clone.inner.app.active_window().as_ref());
                    dialog.set_child(Some(&usbredir.widget()));
                    dialog.present();
                }
            });
            app.inner.app.add_action(&action_usb);
        }
         */
        // TODO
        Err(UsbRedirError("todo".to_string()))
    }
}
