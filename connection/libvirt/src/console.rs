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

use virt::domain::Domain;
use vte::TerminalExt;

use libfieldmonitor::adapter::types::{Adapter, AdapterDisplay};
use libfieldmonitor::connection::ConnectionError;

use crate::connection::VirtArc;

pub struct LibvirtConsoleAdapter {
    domain: VirtArc<Domain>,
}

impl LibvirtConsoleAdapter {
    pub fn new(domain: VirtArc<Domain>) -> Self {
        LibvirtConsoleAdapter { domain }
    }
}

impl LibvirtConsoleAdapter {
    pub const TAG: &'static str = "libvirtcon";
}

impl Adapter for LibvirtConsoleAdapter {
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
    ) -> AdapterDisplay {
        let vte = vte::Terminal::builder()
            .cursor_blink_mode(vte::CursorBlinkMode::On)
            .build();

        AdapterDisplay::Vte(vte)
    }
}
