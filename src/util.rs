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
use adw::gdk;
use std::cmp::Ordering;
use vte::prelude::*;

const BLACK: gdk::RGBA = gdk::RGBA::new(0.0, 0.0, 0.0, 1.0);
const WHITE: gdk::RGBA = gdk::RGBA::new(1.0, 1.0, 1.0, 1.0);

pub const MOUSE_RIGHT_BUTTON: u32 = 3;

pub fn configure_vte_styling(terminal: &vte::Terminal, style_manager: &adw::StyleManager) {
    if style_manager.is_dark() {
        terminal.set_color_foreground(&WHITE);
        terminal.set_color_background(&BLACK);
    } else {
        terminal.set_color_foreground(&BLACK);
        terminal.set_color_background(&WHITE);
    }
}

#[derive(Debug)]
pub enum ListChange<T> {
    Update(T),
    Append,
}

pub struct OrdKeyed<Key, Item>(pub Key, pub Item);

impl<Key: PartialEq, Item> PartialEq for OrdKeyed<Key, Item> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<Key: Eq, Item> Eq for OrdKeyed<Key, Item> {}

impl<Key: PartialOrd, Item> PartialOrd for OrdKeyed<Key, Item> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<Key: Ord, Item> Ord for OrdKeyed<Key, Item> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}
