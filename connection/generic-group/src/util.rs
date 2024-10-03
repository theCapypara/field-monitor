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

use glib::object::{Cast, IsA, ObjectExt};
use gtk::prelude::EditableExt;

/// Clears the text of an Editable if it's `editable` property changes to `false`.
pub fn clear_editable_if_becoming_not_editable(w: &impl IsA<gtk::Editable>) {
    let w = w.upcast_ref();
    w.connect_notify(Some("editable"), move |w, _| {
        if !w.is_editable() {
            w.set_text("")
        }
    });
}
