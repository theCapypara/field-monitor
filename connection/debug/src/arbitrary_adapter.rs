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
use std::time::Duration;

use anyhow::anyhow;
use gtk::prelude::*;

use libfieldmonitor::adapter::types::{Adapter, AdapterDisplay};
use libfieldmonitor::connection::ConnectionError;

use crate::behaviour_preferences::DebugBehaviour;

pub struct DebugArbitraryAdapter {
    pub mode: DebugBehaviour,
    pub overlayed: bool,
}

impl DebugArbitraryAdapter {
    pub const TAG: &'static str = "debugarbitrary";
}

impl Adapter for DebugArbitraryAdapter {
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
    ) -> AdapterDisplay {
        let overlayed = self.overlayed;

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

        AdapterDisplay::Arbitrary {
            widget: gtk::Label::new(Some("Debug Arbitrary Display")).upcast(),
            overlayed,
        }
    }
}
