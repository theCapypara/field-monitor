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

use adw::prelude::*;
use adw::subclass::prelude::*;
use futures::lock::Mutex;
use glib::object::IsA;

use libfieldmonitor::connection::ServerConnection;

use crate::application::FieldMonitorApplication;

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorCLServerEntryExpanderRow)]
    pub struct FieldMonitorCLServerEntryExpanderRow {
        pub server: Mutex<Option<Box<dyn ServerConnection>>>,
        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        #[property(get, construct_only)]
        pub path: RefCell<String>,
        #[property(get, construct_only)]
        pub connection_id: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorCLServerEntryExpanderRow {
        const NAME: &'static str = "FieldMonitorCLServerEntryExpanderRow";
        type Type = super::FieldMonitorCLServerEntryExpanderRow;
        type ParentType = adw::ExpanderRow;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorCLServerEntryExpanderRow {}
    impl WidgetImpl for FieldMonitorCLServerEntryExpanderRow {}
    impl ListBoxRowImpl for FieldMonitorCLServerEntryExpanderRow {}
    impl PreferencesRowImpl for FieldMonitorCLServerEntryExpanderRow {}
    impl ExpanderRowImpl for FieldMonitorCLServerEntryExpanderRow {}
}

glib::wrapper! {
    pub struct FieldMonitorCLServerEntryExpanderRow(ObjectSubclass<imp::FieldMonitorCLServerEntryExpanderRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ExpanderRow;
}

impl super::ServerEntry for FieldMonitorCLServerEntryExpanderRow {
    async fn set_server(&self, server: Box<dyn ServerConnection>) {
        self.imp().server.lock().await.replace(server);
    }

    fn add_suffix(&self, widget: &impl IsA<gtk::Widget>) {
        <Self as ExpanderRowExt>::add_suffix(self, widget)
    }

    fn set_activatable_widget(&self, _widget: Option<&impl IsA<gtk::Widget>>) {
        // no-op / not supported; the expander is activated instead.
    }

    fn path(&self) -> String {
        FieldMonitorCLServerEntryExpanderRow::path(self)
    }

    async fn with_server_if_exists<F>(&self, cb: F)
    where
        F: FnOnce(&dyn ServerConnection),
    {
        let lock = self.imp().server.lock();
        if let Some(locked) = lock.await.as_ref() {
            cb(locked.as_ref())
        }
    }
}
