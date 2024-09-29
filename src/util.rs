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
use adw::gdk;
use vte::prelude::*;

const BLACK: gdk::RGBA = gdk::RGBA::new(0.0, 0.0, 0.0, 1.0);
const WHITE: gdk::RGBA = gdk::RGBA::new(1.0, 1.0, 1.0, 1.0);

pub fn configure_vte_styling(terminal: &vte::Terminal, style_manager: &adw::StyleManager) {
    if style_manager.is_dark() {
        terminal.set_color_foreground(&WHITE);
        terminal.set_color_background(&BLACK);
    } else {
        terminal.set_color_foreground(&BLACK);
        terminal.set_color_background(&WHITE);
    }
}
