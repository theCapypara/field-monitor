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
use std::borrow::Cow;

use gettextrs::gettext;
use glib::prelude::*;

use crate::adapter::types::{Adapter, AdapterDisplay};
use crate::connection::ConnectionError;

pub struct SpiceAdapter {}

impl SpiceAdapter {
    pub const TAG: Cow<'static, str> = Cow::Borrowed("spice");

    pub fn new() -> Self {
        Self {}
    }

    pub fn label() -> Cow<'static, str> {
        gettext("SPICE").into()
    }
}

impl Adapter for SpiceAdapter {
    fn create_and_connect_display(
        self,
        on_connected: &'static dyn Fn(),
        on_disconnected: &'static dyn Fn(Result<(), ConnectionError>),
    ) -> AdapterDisplay {
        let spice = rdw_spice::Display::new();

        todo!();

        AdapterDisplay::Rdw(spice.upcast())
    }
}
