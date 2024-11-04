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
use std::rc::Rc;

use crate::connection::ConnectionError;

/// Widget backing the adapter display.
#[derive(Clone, Debug)]
pub enum AdapterDisplayWidget {
    Rdw(rdw::Display),
    Vte(vte::Terminal),
    Arbitrary {
        widget: gtk::Widget,
        /// If true, the header controls are placed as an overlay over the widget.
        /// If false, they are placed below a visual header bar.
        overlayed: bool,
    },
}

/// A display widget for interacting with the remote server
pub trait AdapterDisplay {
    /// The widget to show the display.
    fn widget(&self) -> AdapterDisplayWidget;

    /// Closes the connection. The widget is still usable afterwards.
    /// Does nothing if the connection is already closed.
    ///
    /// Implementations should also call this in Drop.
    fn close(&self);
}

/// An adapter to connect to a remote server and provide widgets
/// to interact with said server.
pub trait Adapter: Send + Sync {
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
    ) -> Box<dyn AdapterDisplay>;
}
