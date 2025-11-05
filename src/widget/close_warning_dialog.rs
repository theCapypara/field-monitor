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

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FieldMonitorCloseWarningDialog {}

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorCloseWarningDialog {
        const NAME: &'static str = "FieldMonitorCloseWarningDialog";
        type Type = super::FieldMonitorCloseWarningDialog;
        type ParentType = adw::AlertDialog;
    }

    impl ObjectImpl for FieldMonitorCloseWarningDialog {}
    impl WidgetImpl for FieldMonitorCloseWarningDialog {}
    impl AdwDialogImpl for FieldMonitorCloseWarningDialog {}
    impl AdwAlertDialogImpl for FieldMonitorCloseWarningDialog {}
}

glib::wrapper! {
    pub struct FieldMonitorCloseWarningDialog(ObjectSubclass<imp::FieldMonitorCloseWarningDialog>)
        @extends gtk::Widget, adw::Dialog, adw::AlertDialog,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl FieldMonitorCloseWarningDialog {
    pub const RESPONSE_CLOSE: &'static str = "close";

    pub fn new(connection_descriptions: impl IntoIterator<Item = (String, String)>) -> Self {
        let slf: Self = glib::Object::builder()
            .property("heading", gettext("Close Window?"))
            .property("body", gettext("The app is still connected to one or more servers. Closing the window will disconnect from all servers."))
            .build();

        let list_box = gtk::ListBox::builder().css_classes(["boxed-list"]).build();

        for (title, subtitle) in connection_descriptions.into_iter() {
            let label = if subtitle.is_empty() {
                title
            } else {
                format!("{title} - {subtitle}")
            };
            list_box.append(
                &adw::ActionRow::builder()
                    .title(label)
                    .selectable(false)
                    .activatable(false)
                    .focusable(false)
                    .build(),
            )
        }

        slf.set_extra_child(Some(&list_box));

        slf.add_response("cancel", &gettext("Cancel"));
        slf.add_response(Self::RESPONSE_CLOSE, &gettext("Close"));
        slf.set_response_appearance(Self::RESPONSE_CLOSE, adw::ResponseAppearance::Destructive);
        slf.set_default_response(Some("cancel"));
        slf.set_close_response("cancel");

        slf
    }
}
