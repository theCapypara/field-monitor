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
use crate::widget::connection_list::server_info::{
    ServerInfoIcon, ServerInfoUpdater, ServerInfoWidget,
};
use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::WeakRef;
use gtk::glib;
use libfieldmonitor::connection::*;
use std::cell::RefCell;
use std::rc::Rc;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct FieldMonitorServerRow {
        pub prefix: RefCell<Option<WeakRef<ServerInfoIcon>>>,
        pub suffix: RefCell<Option<WeakRef<adw::Bin>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorServerRow {
        const NAME: &'static str = "FieldMonitorServerRow";
        type Type = super::FieldMonitorServerRow;
        type ParentType = adw::ActionRow;
    }

    impl ObjectImpl for FieldMonitorServerRow {}
    impl WidgetImpl for FieldMonitorServerRow {}
    impl ListBoxRowImpl for FieldMonitorServerRow {}
    impl PreferencesRowImpl for FieldMonitorServerRow {}
    impl ActionRowImpl for FieldMonitorServerRow {}
}

glib::wrapper! {
    pub struct FieldMonitorServerRow(ObjectSubclass<imp::FieldMonitorServerRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

impl FieldMonitorServerRow {
    pub async fn new(
        full_path: &[String],
        server: Rc<Box<dyn ServerConnection>>,
    ) -> ConnectionResult<Self> {
        let slf: Self = glib::Object::builder()
            .property("selectable", false)
            .build();
        let imp = slf.imp();

        let prefix = ServerInfoIcon::new();
        slf.add_prefix(&prefix);
        imp.prefix.replace(Some(prefix.downgrade()));
        let suffix = adw::Bin::new();
        slf.add_suffix(&suffix);
        imp.suffix.replace(Some(suffix.downgrade()));

        ServerInfoUpdater::start(slf.downgrade(), server, full_path);

        Ok(slf)
    }
}

impl ServerInfoWidget for FieldMonitorServerRow {
    fn set_server_title(&self, title: &str) {
        self.set_title(title)
    }

    fn set_server_subtitle(&self, subtitle: Option<&str>) {
        self.set_subtitle(subtitle.unwrap_or_default())
    }

    fn get_icon_container(&self) -> ServerInfoIcon {
        self.imp()
            .prefix
            .borrow()
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap()
    }

    fn get_actions_container(&self) -> adw::Bin {
        self.imp()
            .suffix
            .borrow()
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap()
    }

    fn get_row(&self) -> Option<&impl IsA<adw::ActionRow>> {
        Some(self)
    }
}
