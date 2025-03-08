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
use std::rc::Rc;
use std::time::Duration;

use anyhow::anyhow;
use vte::TerminalExt;

use libfieldmonitor::adapter::types::{Adapter, AdapterDisplay, AdapterDisplayWidget};
use libfieldmonitor::connection::ConnectionError;

use crate::behaviour_preferences::DebugBehaviour;

pub struct DebugVteAdapter {
    pub mode: DebugBehaviour,
}

impl DebugVteAdapter {
    pub const TAG: &'static str = "debugvte";
}

impl Adapter for DebugVteAdapter {
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
    ) -> Box<dyn AdapterDisplay> {
        let vte = vte::Terminal::builder()
            .cursor_blink_mode(vte::CursorBlinkMode::On)
            .build();

        vte.feed(b"Hello Debug!");

        glib::timeout_add_local(Duration::from_secs(1), move || {
            match self.mode {
                DebugBehaviour::Ok => {
                    on_connected();
                }
                DebugBehaviour::AuthError => on_disconnected(Err(ConnectionError::AuthFailed(
                    Some("debug auth error".to_string()),
                    anyhow!("debug auth error"),
                ))),
                DebugBehaviour::GeneralError => on_disconnected(Err(ConnectionError::General(
                    Some("debug general error".to_string()),
                    anyhow!("debug general error"),
                ))),
            }
            glib::ControlFlow::Break
        });

        Box::new(DebugVteAdapterDisplay(vte))
    }
}

pub struct DebugVteAdapterDisplay(vte::Terminal);

impl AdapterDisplay for DebugVteAdapterDisplay {
    fn widget(&self) -> AdapterDisplayWidget {
        AdapterDisplayWidget::Vte(self.0.clone())
    }

    fn close(&self) {}
}
