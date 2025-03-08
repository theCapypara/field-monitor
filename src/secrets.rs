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

use std::ops::Deref;

use futures::future::BoxFuture;
use gettextrs::gettext;
use log::warn;
use secure_string::{SecureString, SecureVec};

use libfieldmonitor::ManagesSecrets;

use libfieldmonitor::config::APP_ID;

#[derive(Debug)]
pub struct SecretManager {
    keyring: oo7::portal::Keyring,
}

impl SecretManager {
    pub async fn new() -> anyhow::Result<Self> {
        let keyring = oo7::portal::Keyring::load_default().await?;
        Ok(Self { keyring })
    }
}

impl ManagesSecrets for SecretManager {
    fn lookup(
        &self,
        connection_id: &str,
        field: &str,
    ) -> BoxFuture<anyhow::Result<Option<SecureString>>> {
        let connection_id = connection_id.to_string();
        let field = field.to_string();
        Box::pin(async move {
            let mut attributes = std::collections::HashMap::new();
            attributes.insert("app", APP_ID);
            attributes.insert("connection_id", &connection_id);
            attributes.insert("field", &field);

            let items = self
                .keyring
                .search_items(&attributes)
                .await
                .inspect_err(|err| {
                    warn!("failed to lookup a secret for {connection_id}/{field}: {err}")
                })?;

            match items.first() {
                None => Ok(None),
                Some(item) => {
                    let secret_raw = item.secret();
                    let secret = String::from_utf8(secret_raw.deref().clone())?.into();
                    Ok(Some(secret))
                }
            }
        })
    }

    fn store(
        &self,
        connection_id: &str,
        field: &str,
        password: SecureString,
    ) -> BoxFuture<anyhow::Result<()>> {
        let connection_id = connection_id.to_string();
        let field = field.to_string();
        let password = SecureVec::from(password.unsecure().as_bytes());
        Box::pin(async move {
            let mut attributes = std::collections::HashMap::new();
            attributes.insert("app", APP_ID);
            attributes.insert("connection_id", &connection_id);
            attributes.insert("field", &field);

            self.keyring
                .create_item(
                    &gettext("A secret value used by Field Monitor"),
                    &attributes,
                    password.unsecure(),
                    true,
                )
                .await
                .inspect_err(|err| {
                    warn!("failed to store a secret for {connection_id}/{field}: {err}")
                })
                .map_err(Into::into)
                .map(drop)
        })
    }

    fn clear(&self, connection_id: &str, field: &str) -> BoxFuture<anyhow::Result<()>> {
        let connection_id = connection_id.to_string();
        let field = field.to_string();
        Box::pin(async move {
            let mut attributes = std::collections::HashMap::new();
            attributes.insert("app", APP_ID);
            attributes.insert("connection_id", &connection_id);
            attributes.insert("field", &field);

            self.keyring
                .delete(&attributes)
                .await
                .inspect_err(|err| {
                    warn!("failed to clear a secret for {connection_id}/{field}: {err}")
                })
                .map_err(Into::into)
        })
    }
}
