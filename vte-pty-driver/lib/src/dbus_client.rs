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
use zbus::{Connection, Result, proxy};

#[proxy(
    interface = "de.capypara.FieldMonitor.VtePtyProcMon1",
    default_path = "/de/capypara/FieldMonitor/VtePtyProcMon"
)]
pub trait VtePtyProcMon {
    fn extra_arguments(&self, fm_key: &str) -> Result<Vec<String>>;

    fn set_result(&self, is_err: bool, msg: &str) -> Result<()>;

    fn log_debug(&self, msg: &str) -> Result<()>;

    fn log_error(&self, msg: &str) -> Result<()>;

    fn log_warn(&self, msg: &str) -> Result<()>;
}

pub async fn make_dbus_client(name: &str) -> Result<VtePtyProcMonProxy<'static>> {
    let connection = Connection::session().await?;
    let proxy = VtePtyProcMonProxy::new(&connection, name.to_string()).await?;
    Ok(proxy)
}
