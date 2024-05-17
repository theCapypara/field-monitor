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

use std::collections::HashMap;
use std::mem::take;
use std::sync::Arc;

use futures::future::{LocalBoxFuture, try_join_all};

use crate::ManagesSecrets;

#[derive(Clone)]
pub struct ConnectionConfiguration {
    config: HashMap<String, serde_yaml::Value>,
    provider_tag: String,
    connection_id: String,
    secret_manager: Arc<Box<dyn ManagesSecrets>>,
    pending_secret_changes: HashMap<String, Option<String>>,
}

impl ConnectionConfiguration {
    pub fn new(
        connection_id: String,
        provider_tag: String,
        secret_manager: Arc<Box<dyn ManagesSecrets>>,
    ) -> Self {
        Self {
            config: Default::default(),
            provider_tag,
            connection_id,
            secret_manager,
            pending_secret_changes: Default::default(),
        }
    }

    pub fn new_existing(
        connection_id: String,
        provider_tag: String,
        config: HashMap<String, serde_yaml::Value>,
        secret_manager: Arc<Box<dyn ManagesSecrets>>,
    ) -> Self {
        Self {
            config,
            provider_tag,
            connection_id,
            secret_manager,
            pending_secret_changes: Default::default(),
        }
    }

    pub fn tag(&self) -> &str {
        &self.provider_tag
    }

    pub fn id(&self) -> &str {
        &self.connection_id
    }

    /// Saves pending secret changes to the keychain, returns configuration.
    pub async fn save(&mut self) -> anyhow::Result<HashMap<String, serde_yaml::Value>> {
        let pending_secret_changes = take(&mut self.pending_secret_changes);
        let mut futs: Vec<LocalBoxFuture<anyhow::Result<()>>> =
            Vec::with_capacity(pending_secret_changes.len());
        for (k, v) in pending_secret_changes {
            let secret_manager = self.secret_manager.clone();
            let connection_id = self.connection_id.clone();
            match v {
                None => futs.push(Box::pin(Self::do_clear_secret(
                    secret_manager,
                    connection_id,
                    k,
                ))),
                Some(v) => futs.push(Box::pin(Self::do_set_secret(
                    secret_manager,
                    connection_id,
                    k,
                    v,
                ))),
            }
        }
        try_join_all(futs.into_iter()).await?;
        Ok(self.config.clone())
    }

    pub fn get(&self, key: &str) -> Option<&serde_yaml::Value> {
        self.config.get(key)
    }
    pub fn get_try_as_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }
    pub fn get_try_as_u64(&self, key: &str) -> Option<u64> {
        self.get(key).and_then(|v| v.as_u64())
    }
    pub fn get_try_as_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_i64())
    }
    pub fn set(&mut self, key: impl ToString, value: impl Into<serde_yaml::Value>) {
        self.config.insert(key.to_string(), value.into());
    }
    pub async fn get_secret(&self, key: impl AsRef<str>) -> anyhow::Result<Option<String>> {
        match self.pending_secret_changes.get(key.as_ref()) {
            None => self.do_get_secret(key).await,
            Some(v) => Ok(v.clone()),
        }
    }
    pub fn clear_secret(&mut self, key: impl ToString) {
        self.pending_secret_changes.insert(key.to_string(), None);
    }
    pub fn set_secret(&mut self, key: impl ToString, value: impl ToString) {
        self.pending_secret_changes
            .insert(key.to_string(), Some(value.to_string()));
    }

    async fn do_get_secret(&self, key: impl AsRef<str>) -> anyhow::Result<Option<String>> {
        self.secret_manager
            .lookup(&self.connection_id, key.as_ref())
            .await
            .map(|gstr| gstr.map(Into::into))
            .map_err(Into::into)
    }
    async fn do_clear_secret(
        secret_manager: Arc<Box<dyn ManagesSecrets>>,
        connection_id: String,
        key: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        secret_manager
            .clear(&connection_id, key.as_ref())
            .await
            .map_err(Into::into)
    }
    async fn do_set_secret(
        secret_manager: Arc<Box<dyn ManagesSecrets>>,
        connection_id: String,
        key: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        secret_manager
            .store(&connection_id, key.as_ref(), value.as_ref())
            .await
            .map_err(Into::into)
    }
}
