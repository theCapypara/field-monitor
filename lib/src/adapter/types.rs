/* Copyright 2024-2026 Marco Köpcke
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
use crate::cert_security::{VerifyTls, VerifyTlsResponse};
use crate::connection::ConnectionError;
use glib::object::Cast;
use std::rc::Rc;

/// Widget backing the adapter display.
#[derive(Clone, Debug)]
pub enum AdapterDisplayWidget {
    Rdw(rdw::Display),
    Vte(vte::Terminal),
    Arbitrary { widget: gtk::Widget },
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
        verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> Box<dyn AdapterDisplay>;
}

pub struct NullAdapterDisplay;

impl AdapterDisplay for NullAdapterDisplay {
    fn widget(&self) -> AdapterDisplayWidget {
        AdapterDisplayWidget::Arbitrary {
            widget: gtk::Box::builder().build().upcast(),
        }
    }

    fn close(&self) {}
}
