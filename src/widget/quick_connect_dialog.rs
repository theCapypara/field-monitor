/* Copyright 2024-2025 Marco Köpcke
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

use crate::application::FieldMonitorApplication;
use crate::remote_server_info::RemoteServerInfo;
use crate::widget::window::FieldMonitorWindow;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use log::warn;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorQuickConnectDialog)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/quick_connect_dialog.ui")]
    pub struct FieldMonitorQuickConnectDialog {
        #[template_child]
        pub url_entry: TemplateChild<adw::EntryRow>,
        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        #[property(get, construct_only)]
        pub window: RefCell<Option<FieldMonitorWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorQuickConnectDialog {
        const NAME: &'static str = "FieldMonitorQuickConnectDialog";
        type Type = super::FieldMonitorQuickConnectDialog;
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
    impl ObjectImpl for FieldMonitorQuickConnectDialog {}
    impl WidgetImpl for FieldMonitorQuickConnectDialog {}
    impl AdwDialogImpl for FieldMonitorQuickConnectDialog {}
}

glib::wrapper! {
    pub struct FieldMonitorQuickConnectDialog(ObjectSubclass<imp::FieldMonitorQuickConnectDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl FieldMonitorQuickConnectDialog {
    pub fn new(app: &FieldMonitorApplication, window: &FieldMonitorWindow) -> Self {
        let slf: Self = glib::Object::builder()
            .property("application", app)
            .property("window", window)
            .build();
        slf
    }
}

impl FieldMonitorQuickConnectDialog {
    pub async fn try_connect(&self, file: gio::File) {
        self.set_sensitive(false);
        match RemoteServerInfo::try_from_file(
            file,
            &self.application().unwrap(),
            self.window().as_ref(),
        )
        .await
        {
            Err(err) => {
                let alert = adw::AlertDialog::builder()
                    .heading(gettext("Failed to open"))
                    .body(format!(
                        "{}:\n{}",
                        gettext("Field Monitor could not connect to the server using the specified file or URI"),
                        err
                    ))
                    .build();
                alert.add_response("ok", &gettext("OK"));
                alert.present(self.window().as_ref());
                self.set_sensitive(true);
            }
            Ok(conn) => {
                self.window().unwrap().open_connection_view(conn);
                self.force_close();
            }
        }
    }
}

#[gtk::template_callbacks]
impl FieldMonitorQuickConnectDialog {
    #[template_callback]
    pub async fn on_open_file(&self) {
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        let supported_filter = gtk::FileFilter::new();
        supported_filter.set_name(Some(&gettext("Supported files")));

        let rdp_filter = gtk::FileFilter::new();
        rdp_filter.add_suffix("rdp");
        supported_filter.add_suffix("rdp");
        rdp_filter.add_mime_type("application/x-rdp");
        supported_filter.add_mime_type("application/x-rdp");
        rdp_filter.set_name(Some(&gettext("RDP file")));

        let virtviewer_filter = gtk::FileFilter::new();
        virtviewer_filter.add_suffix("vv");
        supported_filter.add_suffix("vv");
        virtviewer_filter.add_mime_type("application/x-virt-viewer");
        supported_filter.add_mime_type("application/x-virt-viewer");
        virtviewer_filter.set_name(Some(&gettext("Virt Viewer file")));

        let any_filter = gtk::FileFilter::new();
        any_filter.add_pattern("*");
        any_filter.set_name(Some(&gettext("Any file")));

        filters.append(&supported_filter);
        filters.append(&rdp_filter);
        filters.append(&virtviewer_filter);
        filters.append(&any_filter);

        let file_dialog = gtk::FileDialog::builder()
            .title(gettext("Open Connection from file..."))
            .filters(&filters)
            .build();
        let result = file_dialog.open_future(self.window().as_ref()).await;
        match result {
            Ok(file) => self.try_connect(file).await,
            Err(err) => match err.kind::<gtk::DialogError>() {
                Some(gtk::DialogError::Cancelled) | Some(gtk::DialogError::Dismissed) => {}
                _ => {
                    warn!("Failed to open file: {err}");
                    let alert = adw::AlertDialog::builder()
                        .heading(gettext("Failed to open"))
                        .body(gettext(
                            "Field Monitor was unable to open the provided file.",
                        ))
                        .build();
                    alert.add_response("ok", &gettext("OK"));
                    alert.present(self.window().as_ref());
                }
            },
        }
    }

    #[template_callback]
    pub async fn on_connect(&self) {
        self.try_connect(gio::File::for_uri(&self.imp().url_entry.text()))
            .await;
    }
}
