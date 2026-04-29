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

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::Properties;
use std::cell::RefCell;

use crate::widget::usb_redir::DEVICE_DISCONNECT_FAILED;
use crate::widget::usb_redir::settings_dialog::FieldMonitorUsbRedirSettingsDialog;
use libfieldmonitor::adapter::usbredir::{
    FieldMonitorUsbDevice, FieldMonitorUsbRedirAdapter, FieldMonitorUsbRedirAdapterExt,
};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::FieldMonitorUsbRedirSettings)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/usb_redir/settings.ui")]
    pub struct FieldMonitorUsbRedirSettings {
        #[template_child]
        pub list_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub attached_list: TemplateChild<gtk::ListBox>,
        #[property(get, construct_only)]
        pub adapter: RefCell<Option<FieldMonitorUsbRedirAdapter>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorUsbRedirSettings {
        const NAME: &'static str = "FieldMonitorUsbRedirSettings";
        type Type = super::FieldMonitorUsbRedirSettings;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorUsbRedirSettings {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup();
        }
    }

    impl WidgetImpl for FieldMonitorUsbRedirSettings {}
    impl BinImpl for FieldMonitorUsbRedirSettings {}
}

glib::wrapper! {
    pub struct FieldMonitorUsbRedirSettings(ObjectSubclass<imp::FieldMonitorUsbRedirSettings>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl FieldMonitorUsbRedirSettings {
    pub fn new(adapter: &FieldMonitorUsbRedirAdapter) -> Self {
        glib::Object::builder().property("adapter", adapter).build()
    }

    fn setup(&self) {
        let imp = self.imp();
        let adapter = self.adapter().expect("adapter must be set");

        let filter = gtk::CustomFilter::new(|obj| {
            obj.downcast_ref::<FieldMonitorUsbDevice>()
                .is_some_and(|d| d.attached())
        });
        let filter_model = gtk::FilterListModel::new(Some(adapter.store()), Some(filter.clone()));

        Self::connect_attached_changes(&adapter, &filter);

        imp.attached_list.bind_model(
            Some(&filter_model),
            glib::clone!(
                #[weak(rename_to=slf)]
                self,
                #[upgrade_or_panic]
                move |obj| {
                    let device = obj.downcast_ref::<FieldMonitorUsbDevice>().unwrap();
                    slf.build_attached_row(device).upcast()
                }
            ),
        );

        let update_stack = glib::clone!(
            #[weak]
            imp,
            #[weak]
            filter_model,
            move || {
                let name = if filter_model.n_items() == 0 {
                    "empty"
                } else {
                    "list"
                };
                imp.list_stack.set_visible_child_name(name);
            }
        );
        filter_model.connect_items_changed(glib::clone!(
            #[strong]
            update_stack,
            move |_, _, _, _| update_stack()
        ));
        update_stack();
    }

    fn connect_attached_changes(adapter: &FieldMonitorUsbRedirAdapter, filter: &gtk::CustomFilter) {
        let store = adapter.store();
        let connect_device = move |device: &FieldMonitorUsbDevice, filter: &gtk::CustomFilter| {
            device.connect_attached_notify(glib::clone!(
                #[weak]
                filter,
                move |_| {
                    filter.changed(gtk::FilterChange::Different);
                }
            ));
        };
        for item in store.iter::<FieldMonitorUsbDevice>().flatten() {
            connect_device(&item, filter);
        }
        store.connect_items_changed(glib::clone!(
            #[weak]
            filter,
            move |store, position, _removed, added| {
                for i in 0..added {
                    if let Some(device) = store
                        .item(position + i)
                        .and_then(|o| o.downcast::<FieldMonitorUsbDevice>().ok())
                    {
                        connect_device(&device, &filter);
                    }
                }
                filter.changed(gtk::FilterChange::Different);
            }
        ));
    }

    fn build_attached_row(&self, device: &FieldMonitorUsbDevice) -> adw::ActionRow {
        let detach_btn = gtk::Button::builder()
            .icon_name("cross-small-symbolic")
            .tooltip_text(gettext("Disconnect"))
            .valign(gtk::Align::Center)
            .css_classes(["flat", "destructive-action"])
            .build();

        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&device.model()))
            .subtitle(glib::markup_escape_text(&device.vendor()))
            .build();
        row.add_suffix(&detach_btn);

        detach_btn.connect_clicked(glib::clone!(
            #[weak(rename_to=slf)]
            self,
            #[weak]
            device,
            move |btn| {
                btn.set_sensitive(false);
                glib::spawn_future_local(glib::clone!(
                    #[weak]
                    slf,
                    #[weak]
                    device,
                    #[weak]
                    btn,
                    async move {
                        let Some(adapter) = slf.adapter() else {
                            return;
                        };
                        let res = adapter.detach_device(&device).await;
                        btn.set_sensitive(true);
                        if let Err(err) = res {
                            slf.show_error(&DEVICE_DISCONNECT_FAILED, &err.to_string());
                        }
                    }
                ));
            }
        ));

        row
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
impl FieldMonitorUsbRedirSettings {
    #[template_callback]
    fn on_manage_clicked(&self) {
        let Some(adapter) = self.adapter() else {
            return;
        };
        if let Some(popover) = self
            .ancestor(gtk::Popover::static_type())
            .and_downcast::<gtk::Popover>()
        {
            popover.popdown();
        }
        let dialog = FieldMonitorUsbRedirSettingsDialog::new(&adapter);
        dialog.present(self.root().as_ref());
    }
}
