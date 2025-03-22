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
use anyhow::anyhow;
use gettextrs::gettext;
use std::io;
use std::path::PathBuf;

use crate::config::LIBEXECDIR;
use crate::connection::ConnectionError;
pub use secrets::{ManagesSecrets, NullSecretsManager};

#[macro_use]
mod macros;
pub mod adapter;
pub mod busy;
pub mod cache;
pub mod config;
pub mod connection;
pub mod gtk;
pub mod i18n;
mod secrets;
pub mod tokiort;

pub fn config_error(connection_title: Option<String>) -> ConnectionError {
    ConnectionError::General(
        connection_title,
        anyhow!(gettext("The connection configuration is invalid")),
    )
}

pub fn libexec_path(bin_name: &str) -> io::Result<PathBuf> {
    let path = PathBuf::from(LIBEXECDIR).join(bin_name);
    if !path.exists() {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} not found", path.display()),
        ))
    } else {
        Ok(path)
    }
}
