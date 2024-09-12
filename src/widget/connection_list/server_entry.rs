/* Copyright 2024 Marco Köpcke
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
use std::cell::RefCell;
use std::iter;
use std::ops::Deref;

use adw::prelude::*;
use adw::subclass::prelude::*;
use futures::future::try_join_all;
use futures::lock::Mutex;
use gettextrs::gettext;
use gtk::{gio, glib, Widget};
use itertools::Itertools;

use libfieldmonitor::connection::{ConnectionResult, ServerConnection, ServerMap};

use crate::application::FieldMonitorApplication;

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorCLServerEntry)]
    pub struct FieldMonitorCLServerEntry {
        pub connection_id: RefCell<String>,
        pub server_path: RefCell<Vec<String>>,
        pub server: Mutex<Option<Box<dyn ServerConnection>>>,
        #[property(get, set)]
        pub title: RefCell<Option<String>>,
        #[property(get, set)]
        pub subtitle: RefCell<Option<String>>,
        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        #[property(get, construct_only)]
        pub path: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorCLServerEntry {
        const NAME: &'static str = "FieldMonitorCLServerEntry";
        type Type = super::FieldMonitorCLServerEntry;
        type ParentType = adw::Bin;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorCLServerEntry {}
    impl WidgetImpl for FieldMonitorCLServerEntry {}
    impl BinImpl for FieldMonitorCLServerEntry {}
}

glib::wrapper! {
    pub struct FieldMonitorCLServerEntry(ObjectSubclass<imp::FieldMonitorCLServerEntry>)
        @extends gtk::Widget, adw::Bin;
}

impl FieldMonitorCLServerEntry {
    pub fn new(
        app: &FieldMonitorApplication,
        connection_id: String,
        server_path: Vec<String>,
        server: Box<dyn ServerConnection>,
    ) -> Self {
        let path = iter::once(connection_id.as_str())
            .chain(server_path.iter().map(Deref::deref))
            .join("/");
        let metadata = server.metadata();
        let slf: Self = glib::Object::builder()
            .property("application", app)
            .property("path", path)
            .property("title", metadata.title)
            .property("subtitle", metadata.subtitle)
            .build();
        let imp = slf.imp();
        // the lock can not be held at this point, so this can not fail.
        imp.server.try_lock().as_mut().unwrap().replace(server);
        imp.connection_id.replace(connection_id);
        imp.server_path.replace(server_path);
        slf
    }

    /// Actually loads the contents of this widget. This is an asynchronous operation since
    /// potential subservers have to be loaded.
    pub async fn load(&self) -> ConnectionResult<()> {
        let imp = self.imp();
        let server_brw = imp.server.lock().await;
        let server_ref = server_brw.as_ref().unwrap();
        let subservers = server_ref.servers().await?;
        drop(server_brw);

        if subservers.is_empty() {
            self.load_single_server_row().await;
        } else {
            self.load_multi_server_row(subservers).await?;
        }

        Ok(())
    }

    async fn load_single_server_row(&self) {
        let imp = self.imp();
        let self_row = adw::ActionRow::builder()
            .title(imp.title.borrow().clone().unwrap_or_default())
            .subtitle(imp.subtitle.borrow().clone().unwrap_or_default())
            .build();

        self.set_child(Some(&self_row));
        self.bind_property("title", &self_row, "title").build();
        self.bind_property("subtitle", &self_row, "subtitle")
            .build();

        self.finish_load(self_row).await;
    }

    async fn load_multi_server_row(&self, subservers: ServerMap) -> ConnectionResult<()> {
        let imp = self.imp();

        let self_row = adw::ExpanderRow::builder()
            .title(imp.title.borrow().clone().unwrap_or_default())
            .subtitle(imp.subtitle.borrow().clone().unwrap_or_default())
            .build();

        self.set_child(Some(&self_row));
        self.bind_property("title", &self_row, "title").build();
        self.bind_property("subtitle", &self_row, "subtitle")
            .build();

        let mut load_subservers = Vec::with_capacity(subservers.len());

        for (server_id, server) in subservers {
            let server = FieldMonitorCLServerEntry::new(
                &self.application().unwrap(),
                imp.connection_id.borrow().clone(),
                imp.server_path
                    .borrow()
                    .iter()
                    .map(Deref::deref)
                    .chain(iter::once(server_id.as_ref()))
                    .map(ToString::to_string)
                    .collect(),
                server,
            );
            self_row.add_row(&server);
            load_subservers.push(async move { server.load().await });
        }

        try_join_all(load_subservers.into_iter()).await?;

        self.finish_load(self_row).await;

        Ok(())
    }

    async fn finish_load(&self, self_row: impl EitherExpanderOrActionRow) {
        let imp = self.imp();
        let server = imp.server.lock().await;
        if let Some(server) = server.as_ref() {
            let adapters = server.supported_adapters();
            if !adapters.is_empty() {
                let menu = gio::Menu::new();
                for (adapter_id, adapter_label) in adapters {
                    let action_target =
                        (self.path().to_string(), adapter_id.to_string()).to_variant();
                    menu.append(
                        Some(&*adapter_label),
                        Some(
                            gio::Action::print_detailed_name(
                                "app.connect-to-server",
                                Some(&action_target),
                            )
                            .as_str(),
                        ),
                    );
                }
                let menu_button = gtk::MenuButton::builder()
                    .menu_model(&menu)
                    .icon_name("display-with-window-symbolic")
                    .tooltip_text(gettext("Connect"))
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                self_row.add_suffix(&menu_button);
            }
        }
    }
}

trait EitherExpanderOrActionRow {
    fn add_suffix(&self, widget: &impl IsA<gtk::Widget>);
}

impl EitherExpanderOrActionRow for adw::ActionRow {
    fn add_suffix(&self, widget: &impl IsA<Widget>) {
        <Self as ActionRowExt>::add_suffix(self, widget)
    }
}

impl EitherExpanderOrActionRow for adw::ExpanderRow {
    fn add_suffix(&self, widget: &impl IsA<Widget>) {
        <Self as ExpanderRowExt>::add_suffix(self, widget)
    }
}
