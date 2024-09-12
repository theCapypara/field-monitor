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
use crate::connection::ConnectionError;

/// A display widget for interacting with the remote server
pub enum AdapterDisplay {
    Rdw(rdw::Display),
    Vte(vte::Terminal),
}

/// An adapter to connect to a remote server and provide widgets
/// to interact with said server.
pub trait Adapter {
    fn create_and_connect_display(
        self,
        on_connected: &'static dyn Fn(),
        on_disconnected: &'static dyn Fn(Result<(), ConnectionError>),
    ) -> AdapterDisplay;
}
