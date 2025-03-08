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
pub const DEFAULT_GENERIC_ICON: &str = "network-server-symbolic";

mod connection_list_navbar;
mod connection_stack;
mod info_page;
mod server_group;
mod server_info;
mod server_row;

pub use connection_list_navbar::*;
pub use connection_stack::*;
use libfieldmonitor::connection::*;

enum ServerOrConnection<'a> {
    Server(&'a dyn ServerConnection),
    Connection(&'a dyn Connection),
}
