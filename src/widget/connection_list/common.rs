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
use std::borrow::Cow;

use adw::gio;
use adw::prelude::*;
use glib::object::IsA;
use gtk::Widget;

pub(super) trait CanHaveSuffix {
    fn add_suffix(&self, widget: &impl IsA<gtk::Widget>);
}

impl CanHaveSuffix for gtk::Box {
    fn add_suffix(&self, widget: &impl IsA<Widget>) {
        self.append(widget);
    }
}

pub(super) fn add_actions_to_entry(
    row: &impl CanHaveSuffix,
    is_server: bool,
    entity_path: &str,
    actions: Vec<(Cow<str>, Cow<str>)>,
) {
    if actions.is_empty() {
        return;
    }
    let menu = gio::Menu::new();
    for (action_id, action_title) in actions {
        let action_target = (is_server, entity_path, &*action_id).to_variant();
        menu.append(
            Some(&*action_title),
            Some(
                gio::Action::print_detailed_name(
                    "app.perform-connection-action",
                    Some(&action_target),
                )
                .as_str(),
            ),
        );
    }

    let button = gtk::MenuButton::builder()
        .menu_model(&menu)
        .icon_name("view-more-symbolic")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();

    row.add_suffix(&button);
}
