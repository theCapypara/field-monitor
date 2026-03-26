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
pub use hypervisor::*;

mod connection;
mod hypervisor;
mod qemu_preferences;

static LIBVIRT_LOCALHOST: &str = "localhost";

/// Returns `true` if the given localhost is "localhost".
// todo: in the future this might also want to check ip addresses, etc.
#[inline(always)]
pub fn is_localhost(hostname: &str) -> bool {
    hostname == LIBVIRT_LOCALHOST
}
