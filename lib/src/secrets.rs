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
use futures::future::BoxFuture;
use secure_string::SecureString;

pub trait ManagesSecrets: Send + Sync {
    fn lookup(
        &self,
        connection_id: &str,
        field: &str,
    ) -> BoxFuture<anyhow::Result<Option<SecureString>>>;
    fn store(
        &self,
        connection_id: &str,
        field: &str,
        password: SecureString,
    ) -> BoxFuture<anyhow::Result<()>>;
    fn clear(&self, connection_id: &str, field: &str) -> BoxFuture<anyhow::Result<()>>;
}

pub struct NullSecretsManager;

impl ManagesSecrets for NullSecretsManager {
    fn lookup(
        &self,
        _connection_id: &str,
        _field: &str,
    ) -> BoxFuture<anyhow::Result<Option<SecureString>>> {
        Box::pin(async move { Ok(None) })
    }

    fn store(
        &self,
        _connection_id: &str,
        _field: &str,
        _password: SecureString,
    ) -> BoxFuture<anyhow::Result<()>> {
        Box::pin(async move { Ok(()) })
    }

    fn clear(&self, _connection_id: &str, _field: &str) -> BoxFuture<anyhow::Result<()>> {
        Box::pin(async move { Ok(()) })
    }
}
