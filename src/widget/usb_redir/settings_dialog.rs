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

use crate::widget::usb_redir::{
    DEVICE_CONNECT_FAILED, DEVICE_CONNECTED, DEVICE_DISCONNECT_FAILED, DEVICE_DISCONNECTED,
    NO_DEVICES_AVAILABLE,
};
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::Properties;
use libfieldmonitor::adapter::usbredir::{
    FieldMonitorUsbDevice, FieldMonitorUsbRedirAdapter, FieldMonitorUsbRedirAdapterExt,
};
use libfieldmonitor::i18n::gettext_f;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::FieldMonitorUsbRedirSettingsDialog)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/usb_redir/settings_dialog.ui")]
    pub struct FieldMonitorUsbRedirSettingsDialog {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub devices_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub show_all_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub free_channels_label: TemplateChild<gtk::Label>,
        #[property(get, construct_only)]
        pub adapter: RefCell<Option<FieldMonitorUsbRedirAdapter>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorUsbRedirSettingsDialog {
        const NAME: &'static str = "FieldMonitorUsbRedirSettingsDialog";
        type Type = super::FieldMonitorUsbRedirSettingsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorUsbRedirSettingsDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup();
        }
    }

    impl WidgetImpl for FieldMonitorUsbRedirSettingsDialog {}
    impl AdwDialogImpl for FieldMonitorUsbRedirSettingsDialog {}
}

glib::wrapper! {
    pub struct FieldMonitorUsbRedirSettingsDialog(ObjectSubclass<imp::FieldMonitorUsbRedirSettingsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl FieldMonitorUsbRedirSettingsDialog {
    pub fn new(adapter: &FieldMonitorUsbRedirAdapter) -> Self {
        glib::Object::builder().property("adapter", adapter).build()
    }

    fn setup(&self) {
        let imp = self.imp();
        let adapter = self.adapter().expect("adapter must be set");

        let show_all_row = imp.show_all_row.clone();
        let filter = gtk::CustomFilter::new(glib::clone!(
            #[weak]
            show_all_row,
            #[upgrade_or]
            true,
            move |obj| {
                let device = match obj.downcast_ref::<FieldMonitorUsbDevice>() {
                    Some(d) => d,
                    None => return false,
                };
                show_all_row.is_active() || !device.hub()
            }
        ));
        imp.show_all_row.connect_active_notify(glib::clone!(
            #[weak]
            filter,
            move |_| filter.changed(gtk::FilterChange::Different)
        ));

        let filter_model = gtk::FilterListModel::new(Some(adapter.store()), Some(filter));

        imp.devices_group.bind_model(
            Some(&filter_model),
            glib::clone!(
                #[weak(rename_to=slf)]
                self,
                #[weak]
                adapter,
                #[upgrade_or_panic]
                move |obj| {
                    let device = obj.downcast_ref::<FieldMonitorUsbDevice>().unwrap();
                    slf.build_device_row(&adapter, device).upcast()
                }
            ),
        );

        let update_description = glib::clone!(
            #[weak]
            imp,
            move |filter_model: &gtk::FilterListModel| {
                if filter_model.n_items() == 0 {
                    imp.devices_group
                        .set_description(Some(&NO_DEVICES_AVAILABLE));
                } else {
                    imp.devices_group.set_description(None);
                }
            }
        );
        filter_model.connect_items_changed(glib::clone!(
            #[strong]
            update_description,
            move |filter_model, _, _, _| update_description(filter_model)
        ));
        update_description(&filter_model);

        // XXX: gettext's scanning can't pick up the string in the glib::clone closure,
        //      so we use this pattern instead and pass imp in the closures below.
        let update_label =
            move |adapter: &FieldMonitorUsbRedirAdapter,
                  imp: &imp::FieldMonitorUsbRedirSettingsDialog| {
                let max = adapter.max_channels();
                if max < 0 {
                    imp.free_channels_label.set_visible(false);
                } else {
                    let free = adapter.free_channels();
                    let text = if free == 1 {
                        gettext("1 channel free")
                    } else {
                        // Translators: {free} = number of free USB redirection channels
                        gettext_f("{free} channels free", &[("free", &free.to_string())])
                    };
                    imp.free_channels_label.set_label(&text);
                    imp.free_channels_label.set_visible(true);
                }
            };
        adapter.connect_free_channels_notify(glib::clone!(
            #[weak]
            imp,
            move |adapter| update_label(adapter, &imp)
        ));
        adapter.connect_max_channels_notify(glib::clone!(
            #[weak]
            imp,
            move |adapter| update_label(adapter, &imp)
        ));
        update_label(&adapter, imp);
    }

    fn build_device_row(
        &self,
        adapter: &FieldMonitorUsbRedirAdapter,
        device: &FieldMonitorUsbDevice,
    ) -> adw::ActionRow {
        let spinner = adw::Spinner::builder()
            .visible(false)
            .valign(gtk::Align::Center)
            // todo: this is the padding inside the buttons, we probably shouldn't hardcode it like this
            .margin_end(10)
            .build();

        let warning_icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
        warning_icon.add_css_class("warning");
        warning_icon.set_tooltip_text(Some(&gettext(
            "This device can not be connected, you might be missing permissions.",
        )));

        let attach_btn = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text(gettext("Connect"))
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();

        let detach_btn = gtk::Button::builder()
            .icon_name("cross-small-symbolic")
            .tooltip_text(gettext("Disconnect"))
            .valign(gtk::Align::Center)
            .css_classes(["flat", "destructive-action"])
            .build();

        device
            .bind_property("attached", &detach_btn, "visible")
            .sync_create()
            .build();
        device
            .bind_property("attached", &attach_btn, "visible")
            .sync_create()
            .invert_boolean()
            .build();
        device
            .bind_property("attachable", &warning_icon, "visible")
            .invert_boolean()
            .sync_create()
            .build();
        device
            .bind_property("attachable", &attach_btn, "sensitive")
            .sync_create()
            .build();

        let update_attachable_from_free_channels = glib::clone!(
            #[weak]
            attach_btn,
            #[weak]
            device,
            move |adapter: &FieldMonitorUsbRedirAdapter| {
                if adapter.max_channels() >= 0 && adapter.free_channels() == 0 {
                    attach_btn.set_sensitive(false);
                } else {
                    // restore state based on attachable state
                    device.notify_attachable();
                }
            }
        );
        adapter.connect_free_channels_notify(glib::clone!(
            #[strong]
            update_attachable_from_free_channels,
            move |adapter| update_attachable_from_free_channels(adapter),
        ));
        update_attachable_from_free_channels(adapter);

        let suffix = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .build();
        suffix.append(&warning_icon);
        suffix.append(&spinner);
        suffix.append(&attach_btn);
        suffix.append(&detach_btn);

        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&device.model()))
            .subtitle(glib::markup_escape_text(&device.vendor()))
            .build();
        row.add_suffix(&suffix);

        attach_btn.connect_clicked(glib::clone!(
            #[weak(rename_to=slf)]
            self,
            #[weak]
            device,
            #[weak]
            spinner,
            #[weak]
            attach_btn,
            #[weak]
            detach_btn,
            move |_| {
                slf.spawn_attach(device, spinner, attach_btn, detach_btn);
            }
        ));

        detach_btn.connect_clicked(glib::clone!(
            #[weak(rename_to=slf)]
            self,
            #[weak]
            device,
            #[weak]
            spinner,
            #[weak]
            attach_btn,
            #[weak]
            detach_btn,
            move |_| {
                slf.spawn_detach(device, spinner, attach_btn, detach_btn);
            }
        ));

        row
    }

    fn spawn_attach(
        &self,
        device: FieldMonitorUsbDevice,
        spinner: adw::Spinner,
        attach_btn: gtk::Button,
        detach_btn: gtk::Button,
    ) {
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to=slf)]
            self,
            async move {
                let Some(adapter) = slf.adapter() else {
                    return;
                };
                attach_btn.set_visible(false);
                detach_btn.set_visible(false);
                spinner.set_visible(true);

                let parent_window = slf.root().and_downcast::<gtk::Window>();
                let res = adapter.attach_device(&device, parent_window.as_ref()).await;

                spinner.set_visible(false);
                // Bindings restore the correct button visibility from the device state.
                device.notify_attached();
                device.notify_attachable();

                match res {
                    Ok(()) => slf.toast(&DEVICE_CONNECTED),
                    Err(err) => slf.show_error(&DEVICE_CONNECT_FAILED, &err.to_string()),
                }
            }
        ));
    }

    fn spawn_detach(
        &self,
        device: FieldMonitorUsbDevice,
        spinner: adw::Spinner,
        attach_btn: gtk::Button,
        detach_btn: gtk::Button,
    ) {
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to=slf)]
            self,
            async move {
                let Some(adapter) = slf.adapter() else {
                    return;
                };
                attach_btn.set_visible(false);
                detach_btn.set_visible(false);
                spinner.set_visible(true);

                let res = adapter.detach_device(&device).await;

                spinner.set_visible(false);
                device.notify_attached();
                device.notify_attachable();

                match res {
                    Ok(()) => slf.toast(&DEVICE_DISCONNECTED),
                    Err(err) => slf.show_error(&DEVICE_DISCONNECT_FAILED, &err.to_string()),
                }
            }
        ));
    }

    fn toast(&self, msg: &str) {
        self.imp()
            .toast_overlay
            .add_toast(adw::Toast::builder().title(msg).timeout(5).build());
    }

    fn show_error(&self, heading: &str, body: &str) {
        let dialog = adw::AlertDialog::builder()
            .heading(heading)
            .body(body)
            .build();
        dialog.add_response("ok", &gettext("OK"));
        dialog.set_default_response(Some("ok"));
        dialog.set_close_response("ok");
        dialog.present(self.root().as_ref());
    }
}

#[gtk::template_callbacks]
impl FieldMonitorUsbRedirSettingsDialog {
    #[template_callback]
    fn on_help_clicked(&self) {
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("USB Devices"))
            .body_use_markup(true)
            .body(gettext("This dialog allows you to share USB devices with the remote server or virtual machine.\n\nThe number of devices that can be shared may be limited by the remote server.\n\nThe list hides some devices (USB hubs, etc.) by default. Use the toggle button to show all devices, but note that USB hubs can not be properly shared with remote servers.\n\nTo share a device your user account may need read and write permissions on the raw device, consult your distribution documentation for more information."))
            .build();
        dialog.add_response("ok", &gettext("OK"));
        dialog.set_default_response(Some("ok"));
        dialog.set_close_response("ok");
        dialog.present(self.root().as_ref());
    }
}
