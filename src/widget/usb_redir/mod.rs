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
use gettextrs::gettext;
use std::sync::LazyLock;

// XXX: These are here because (1) some of them we need in multiple places and (2) they are used inside
//      glib::clone where they can't be picked up by gettext's scanner
static DEVICE_CONNECTED: LazyLock<String> = LazyLock::new(|| gettext("USB device connected"));
static DEVICE_CONNECT_FAILED: LazyLock<String> =
    LazyLock::new(|| gettext("Failed to connect device"));
static DEVICE_DISCONNECTED: LazyLock<String> = LazyLock::new(|| gettext("USB device disconnected"));
static DEVICE_DISCONNECT_FAILED: LazyLock<String> =
    LazyLock::new(|| gettext("Failed to disconnect device"));
static NO_DEVICES_AVAILABLE: LazyLock<String> = LazyLock::new(|| gettext("No devices available."));

pub mod settings;
pub mod settings_dialog;
