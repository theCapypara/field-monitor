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
use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::Runtime;

pub fn tkruntime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("failed setting up tokio async runtime"))
}

pub async fn run_on_tokio<F, T, E>(fut: F) -> Result<T, E>
where
    F: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: From<anyhow::Error> + Send + 'static,
{
    tkruntime()
        .spawn(fut)
        .await
        .map_err(|err| anyhow::Error::from(err).context("tokio join failed").into())
        .and_then(|r| r) // flatten
}
