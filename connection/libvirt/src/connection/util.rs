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
use log::error;
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixStream;
use virt::domain::Domain;
use virt::error::Error;
use virt::sys::VIR_DOMAIN_OPEN_GRAPHICS_SKIPAUTH;

pub fn open_libvirt_fd_stream(domain: &Domain, graphics_idx: usize) -> Result<UnixStream, Error> {
    domain
        .open_graphics_fd(graphics_idx as _, VIR_DOMAIN_OPEN_GRAPHICS_SKIPAUTH)
        // SAFETY: If open_graphics_fd doesn't error, the fd points to a valid file descriptor.
        .map(|fd| unsafe { UnixStream::from_raw_fd(fd as _) })
        .inspect_err(|err| error!("libvirt openGraphicsFd failed: {err}"))
}
