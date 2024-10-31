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
use crate::widget::connection_list::make_server_prefix_suffix;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use libfieldmonitor::connection::*;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct FieldMonitorServerRow {}

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
        server: Box<dyn ServerConnection>,
    ) -> ConnectionResult<Self> {
        let metadata = server.metadata();
        let slf: Self = glib::Object::builder()
            .property("title", &metadata.title)
            .property("subtitle", &metadata.subtitle)
            .property("selectable", false)
            .build();

        let (prefix, suffix) =
            make_server_prefix_suffix(server.as_ref(), full_path, Some(&slf)).await?;
        slf.add_prefix(&prefix);
        slf.add_suffix(&suffix);

        Ok(slf)
    }
}
