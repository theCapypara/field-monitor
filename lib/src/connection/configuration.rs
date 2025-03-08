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
#![allow(clippy::arc_with_non_send_sync)] // future proofing

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::mem::take;
use std::sync::Arc;

use futures::future::{try_join_all, BoxFuture, LocalBoxFuture};
use secure_string::SecureString;
use serde_yaml::{Mapping, Value};

use crate::connection::config_value::{ConfigValue, ConfigValueRef};
use crate::ManagesSecrets;

pub trait ConfigAccess {
    fn get(&self, key: &str) -> Option<ConfigValueRef>;
    fn get_try_as_str(&self, key: &str) -> Option<&str> {
        self.get(key)
            .and_then(|v| v.as_serde_value().and_then(Value::as_str))
    }
    fn get_try_as_string(&self, key: &str) -> Option<String> {
        self.get_try_as_str(key).map(ToString::to_string)
    }
    fn get_try_as_sec_string(&self, key: &str) -> Option<SecureString> {
        self.get(key).and_then(|v| match v {
            ConfigValueRef::SecureString(v) => Some(v.clone()),
            _ => None,
        })
    }
    fn get_try_as_u32(&self, key: &str) -> Option<u32> {
        self.get_try_as_u64(key).and_then(|v| v.try_into().ok())
    }
    fn get_try_as_u64(&self, key: &str) -> Option<u64> {
        self.get(key)
            .and_then(|v| v.as_serde_value().and_then(Value::as_u64))
    }
    fn get_try_as_i64(&self, key: &str) -> Option<i64> {
        self.get(key)
            .and_then(|v| v.as_serde_value().and_then(Value::as_i64))
    }
    fn get_try_as_bool(&self, key: &str) -> Option<bool> {
        self.get(key)
            .and_then(|v| v.as_serde_value().and_then(Value::as_bool))
    }
    fn get_secret(&self, key: impl ToString) -> BoxFuture<anyhow::Result<Option<SecureString>>>;
}

pub trait ConfigAccessMut {
    fn clear(&mut self, key: impl AsRef<str>);
    /// Sets a value in the config. Keys starting with __ are not persisted in `save`.
    fn set_value(&mut self, key: impl ToString, value: impl Into<Value>);
    /// Sets a secure string in the config. These are never persisted in `save`.
    fn set_secure_string(&mut self, key: impl ToString, value: impl Into<SecureString>);
    fn clear_secret(&mut self, key: impl ToString);
    /// Set a secret in the config. These are never serialized, however they are saved and restored
    /// from the secret service when calling `save`.
    fn set_secret(&mut self, key: impl ToString, value: SecureString);
}

#[derive(Clone)]
pub struct ConnectionConfiguration {
    config: HashMap<String, Value>,
    config_not_persisted: HashMap<String, ConfigValue>,
    provider_tag: String,
    connection_id: String,
    secret_manager: Arc<Box<dyn ManagesSecrets>>,
    pending_secret_changes: HashMap<String, Option<SecureString>>,
}

impl ConnectionConfiguration {
    pub fn new(
        connection_id: String,
        provider_tag: String,
        secret_manager: Arc<Box<dyn ManagesSecrets>>,
    ) -> Self {
        Self {
            config: Default::default(),
            config_not_persisted: Default::default(),
            provider_tag,
            connection_id,
            secret_manager,
            pending_secret_changes: Default::default(),
        }
    }

    pub fn new_existing(
        connection_id: String,
        provider_tag: String,
        config: HashMap<String, Value>,
        secret_manager: Arc<Box<dyn ManagesSecrets>>,
    ) -> Self {
        Self {
            config,
            config_not_persisted: Default::default(),
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

    /// Saves pending secret changes to the keychain, returns persistent configuration.
    pub async fn save(&mut self) -> anyhow::Result<HashMap<String, Value>> {
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
        value: SecureString,
    ) -> anyhow::Result<()> {
        secret_manager
            .store(&connection_id, key.as_ref(), value)
            .await
            .map_err(Into::into)
    }

    pub fn with_section<'a, 'b, F, T>(&'a self, section_key: &'b str, cb: F) -> T
    where
        F: FnOnce(ConfigSectionRef<'b>) -> T,
        T: 'a,
        'a: 'b,
    {
        match self.config.get(section_key) {
            Some(Value::Mapping(section_map)) => cb(ConfigSectionRef {
                connection_id: &self.connection_id,
                section_key,
                section_map: Some(section_map),
                config_not_persisted: &self.config_not_persisted,
                secret_manager: self.secret_manager.as_ref().as_ref(),
                pending_secret_changes: &self.pending_secret_changes,
            }),
            _ => cb(ConfigSectionRef {
                connection_id: &self.connection_id,
                section_key,
                section_map: None,
                config_not_persisted: &self.config_not_persisted,
                secret_manager: self.secret_manager.as_ref().as_ref(),
                pending_secret_changes: &self.pending_secret_changes,
            }),
        }
    }

    pub fn with_section_mut<F, T>(&mut self, section_key: &str, cb: F) -> T
    where
        F: FnOnce(ConfigSectionMut) -> T,
        T: 'static,
    {
        match self.config.entry(section_key.to_string()) {
            Entry::Occupied(mut entry) => match entry.get_mut() {
                Value::Mapping(section_map) => cb(ConfigSectionMut {
                    connection_id: &self.connection_id,
                    section_key,
                    section_map,
                    config_not_persisted: &mut self.config_not_persisted,
                    secret_manager: self.secret_manager.as_ref().as_ref(),
                    pending_secret_changes: &mut self.pending_secret_changes,
                }),
                v => {
                    *v = Value::Mapping(Mapping::new());
                    match v {
                        Value::Mapping(section_map) => cb(ConfigSectionMut {
                            connection_id: &self.connection_id,
                            section_key,
                            section_map,
                            config_not_persisted: &mut self.config_not_persisted,
                            secret_manager: self.secret_manager.as_ref().as_ref(),
                            pending_secret_changes: &mut self.pending_secret_changes,
                        }),
                        _ => unreachable!(),
                    }
                }
            },
            Entry::Vacant(entry) => match entry.insert(Value::Mapping(Mapping::new())) {
                Value::Mapping(section_map) => cb(ConfigSectionMut {
                    connection_id: &self.connection_id,
                    section_key,
                    section_map,
                    config_not_persisted: &mut self.config_not_persisted,
                    secret_manager: self.secret_manager.as_ref().as_ref(),
                    pending_secret_changes: &mut self.pending_secret_changes,
                }),
                _ => unreachable!(),
            },
        }
    }

    pub async fn with_section_async<'a, 'b, F, T>(&'a self, section_key: &'b str, cb: F) -> T
    where
        F: FnOnce(ConfigSectionRef<'b>) -> LocalBoxFuture<'b, T>,
        T: 'static,
        'a: 'b,
    {
        async move {
            match self.config.get(section_key) {
                Some(Value::Mapping(section_map)) => {
                    cb(ConfigSectionRef {
                        connection_id: &self.connection_id,
                        section_key,
                        section_map: Some(section_map),
                        config_not_persisted: &self.config_not_persisted,
                        secret_manager: self.secret_manager.as_ref().as_ref(),
                        pending_secret_changes: &self.pending_secret_changes,
                    })
                    .await
                }
                _ => {
                    cb(ConfigSectionRef {
                        connection_id: &self.connection_id,
                        section_key,
                        section_map: None,
                        config_not_persisted: &self.config_not_persisted,
                        secret_manager: self.secret_manager.as_ref().as_ref(),
                        pending_secret_changes: &self.pending_secret_changes,
                    })
                    .await
                }
            }
        }
        .await
    }

    pub fn for_each_section<F>(&self, mut cb: F)
    where
        F: FnMut(&str, ConfigSectionRef),
    {
        for (k, v) in &self.config {
            if let Value::Mapping(map) = v {
                cb(
                    k,
                    ConfigSectionRef {
                        connection_id: &self.connection_id,
                        section_key: k,
                        section_map: Some(map),
                        config_not_persisted: &self.config_not_persisted,
                        secret_manager: self.secret_manager.as_ref().as_ref(),
                        pending_secret_changes: &self.pending_secret_changes,
                    },
                )
            }
        }
    }

    pub fn section_keys(&self) -> impl Iterator<Item = &str> + '_ {
        self.config.iter().filter_map(|(k, v)| {
            if let Value::Mapping(_) = v {
                Some(k.as_str())
            } else {
                None
            }
        })
    }
}

impl ConfigAccess for ConnectionConfiguration {
    fn get(&self, key: &str) -> Option<ConfigValueRef> {
        if key.starts_with("__") {
            self.config_not_persisted.get(key).map(Into::into)
        } else {
            self.config.get(key).map(ConfigValueRef::SerdeValue)
        }
    }

    fn get_secret(&self, key: impl ToString) -> BoxFuture<anyhow::Result<Option<SecureString>>> {
        let key = key.to_string();
        Box::pin(async move {
            match self.pending_secret_changes.get(&key) {
                None => self
                    .secret_manager
                    .lookup(&self.connection_id, key.as_ref())
                    .await
                    .map(|gstr| gstr.map(Into::into))
                    .map_err(Into::into),
                Some(v) => Ok(v.clone()),
            }
        })
    }
}

impl ConfigAccessMut for ConnectionConfiguration {
    fn clear(&mut self, key: impl AsRef<str>) {
        self.config.remove(key.as_ref());
    }
    fn set_value(&mut self, key: impl ToString, value: impl Into<Value>) {
        let key = key.to_string();
        if key.starts_with("__") {
            self.config_not_persisted.insert(key, value.into().into());
        } else {
            self.config.insert(key, value.into());
        }
    }
    fn set_secure_string(&mut self, key: impl ToString, value: impl Into<SecureString>) {
        self.config_not_persisted
            .insert(key.to_string(), value.into().into());
    }
    fn clear_secret(&mut self, key: impl ToString) {
        self.pending_secret_changes.insert(key.to_string(), None);
    }
    fn set_secret(&mut self, key: impl ToString, value: SecureString) {
        self.pending_secret_changes
            .insert(key.to_string(), Some(value));
    }
}

pub struct ConfigSectionRef<'a> {
    connection_id: &'a str,
    section_key: &'a str,
    section_map: Option<&'a Mapping>,
    config_not_persisted: &'a HashMap<String, ConfigValue>,
    secret_manager: &'a dyn ManagesSecrets,
    pending_secret_changes: &'a HashMap<String, Option<SecureString>>,
}

pub struct ConfigSectionMut<'a> {
    connection_id: &'a str,
    section_key: &'a str,
    section_map: &'a mut Mapping,
    config_not_persisted: &'a mut HashMap<String, ConfigValue>,
    secret_manager: &'a dyn ManagesSecrets,
    pending_secret_changes: &'a mut HashMap<String, Option<SecureString>>,
}

impl ConfigAccess for ConfigSectionRef<'_> {
    fn get(&self, key: &str) -> Option<ConfigValueRef> {
        if key.starts_with("__") {
            self.config_not_persisted
                .get(&format!("{}///{}", self.section_key, key))
                .map(Into::into)
        } else {
            self.section_map
                .and_then(|section_map| section_map.get(key).map(ConfigValueRef::SerdeValue))
        }
    }

    fn get_secret(&self, key: impl ToString) -> BoxFuture<anyhow::Result<Option<SecureString>>> {
        let key = key.to_string();
        Box::pin(async move {
            let key = format!("{}///{}", self.section_key, &key);
            match self.pending_secret_changes.get(&key) {
                None => self
                    .secret_manager
                    .lookup(self.connection_id, &key)
                    .await
                    .map(|gstr| gstr.map(Into::into))
                    .map_err(Into::into),
                Some(v) => Ok(v.clone()),
            }
        })
    }
}

impl ConfigAccess for ConfigSectionMut<'_> {
    fn get(&self, key: &str) -> Option<ConfigValueRef> {
        if key.starts_with("__") {
            self.config_not_persisted
                .get(&format!("{}///{}", self.section_key, key))
                .map(Into::into)
        } else {
            self.section_map.get(key).map(ConfigValueRef::SerdeValue)
        }
    }

    fn get_secret(&self, key: impl ToString) -> BoxFuture<anyhow::Result<Option<SecureString>>> {
        let key = key.to_string();
        Box::pin(async move {
            let key = format!("{}///{}", self.section_key, &key);
            match self.pending_secret_changes.get(&key) {
                None => self
                    .secret_manager
                    .lookup(self.connection_id, &key)
                    .await
                    .map(|gstr| gstr.map(Into::into))
                    .map_err(Into::into),
                Some(v) => Ok(v.clone()),
            }
        })
    }
}

impl ConfigAccessMut for ConfigSectionMut<'_> {
    fn clear(&mut self, key: impl AsRef<str>) {
        self.section_map.remove(key.as_ref());
    }

    fn set_value(&mut self, key: impl ToString, value: impl Into<Value>) {
        let key = key.to_string();
        if key.starts_with("__") {
            self.config_not_persisted.insert(
                format!("{}///{}", self.section_key, key),
                value.into().into(),
            );
        } else {
            self.section_map
                .insert(Value::String(key.to_string()), value.into());
        }
    }

    fn set_secure_string(&mut self, key: impl ToString, value: impl Into<SecureString>) {
        self.config_not_persisted.insert(
            format!("{}///{}", self.section_key, key.to_string()),
            value.into().into(),
        );
    }

    fn clear_secret(&mut self, key: impl ToString) {
        self.pending_secret_changes
            .insert(format!("{}///{}", self.section_key, key.to_string()), None);
    }

    fn set_secret(&mut self, key: impl ToString, value: SecureString) {
        self.pending_secret_changes.insert(
            format!("{}///{}", self.section_key, key.to_string()),
            Some(value),
        );
    }
}

/// A wrapper struct that wraps configuration for a session and persistent configuration.
/// These may be the same, but they don't have to be. Session configuration is to be used
/// temporarily, at most as long as the app is running, while persistent configuration can be
/// saved to disk and re-used when later loading a connection again.
pub struct DualScopedConnectionConfiguration {
    session: Arc<ConnectionConfiguration>,
    persistent: Arc<ConnectionConfiguration>,
}

const DUAL_ERR: &str = "expected DualScopedConnectionConfiguration inner Arc to not be shared";

impl DualScopedConnectionConfiguration {
    /// Create a new instance, where both the session and persistent configurations are the same.
    pub fn new_unified(config: ConnectionConfiguration) -> Self {
        let arc = Arc::new(config);
        Self {
            session: arc.clone(),
            persistent: arc,
        }
    }

    /// Create a new instance where the session and persistent configurations are different.
    pub fn new_separate(
        session: ConnectionConfiguration,
        persistent: ConnectionConfiguration,
    ) -> Self {
        assert_eq!(&session.connection_id, &persistent.connection_id);
        assert_eq!(&session.provider_tag, &persistent.provider_tag);
        match Self::collapse_configs_if_effectively_eq(session, persistent) {
            Ok(c) => Self::new_unified(c),
            Err((session, persistent)) => Self {
                session: Arc::new(session),
                persistent: Arc::new(persistent),
            },
        }
    }

    pub fn session(&self) -> &ConnectionConfiguration {
        &self.session
    }

    pub fn persistent(&self) -> &ConnectionConfiguration {
        &self.persistent
    }

    pub fn persistent_mut(&mut self) -> &mut ConnectionConfiguration {
        if Arc::ptr_eq(&self.persistent, &self.session) && Arc::strong_count(&self.persistent) == 2
        {
            // TODO: Arc::get_mut_unchecked would also be possible here and of course be much
            //       better performing, but it's unstable.
            self.session = Arc::new((*self.persistent).clone());
        }

        // This SHOULD be okay, since the only other places we call get_mut are in methods that
        // consume self.
        Arc::get_mut(&mut self.persistent).expect(DUAL_ERR)
    }

    /// Apply an update to both the session and persistent configuration and return a new instance
    /// of Self with the data.
    pub fn transform_update_unified<F, E>(self, f: F) -> Result<Self, E>
    where
        F: Fn(&mut ConnectionConfiguration) -> Result<(), E>,
    {
        let (c_session, c_persistent) = (self.session, self.persistent);

        if Arc::ptr_eq(&c_session, &c_persistent) {
            drop(c_persistent);
            let mut c_session = Arc::into_inner(c_session).expect(DUAL_ERR);
            f(&mut c_session)?;
            let arc = Arc::new(c_session);
            Ok(Self {
                session: arc.clone(),
                persistent: arc,
            })
        } else {
            let mut c_session = Arc::into_inner(c_session).expect(DUAL_ERR);
            let mut c_persistent = Arc::into_inner(c_persistent).expect(DUAL_ERR);
            f(&mut c_session)?;
            f(&mut c_persistent)?;
            Ok(Self {
                session: Arc::new(c_session),
                persistent: Arc::new(c_persistent),
            })
        }
    }

    /// Apply a different update to each the session and persistent configuration and return
    /// a new instance of Self with the data.
    pub fn transform_update_separate<F1, F2, E>(
        self,
        f_session: F1,
        f_persistent: F2,
    ) -> Result<Self, E>
    where
        F1: Fn(&mut ConnectionConfiguration) -> Result<(), E>,
        F2: Fn(&mut ConnectionConfiguration) -> Result<(), E>,
    {
        let (c_session, c_persistent) = (self.session, self.persistent);

        let (mut c_session, mut c_persistent) = if Arc::ptr_eq(&c_session, &c_persistent) {
            drop(c_persistent);
            let inner = Arc::into_inner(c_session).expect(DUAL_ERR);
            (inner.clone(), inner)
        } else {
            (
                Arc::into_inner(c_session).expect(DUAL_ERR),
                Arc::into_inner(c_persistent).expect(DUAL_ERR),
            )
        };

        f_session(&mut c_session)?;
        f_persistent(&mut c_persistent)?;

        match Self::collapse_configs_if_effectively_eq(c_session, c_persistent) {
            Ok(c) => {
                let arc = Arc::new(c);
                Ok(Self {
                    session: arc.clone(),
                    persistent: arc,
                })
            }
            Err((c_session, c_persistent)) => Ok(Self {
                session: Arc::new(c_session),
                persistent: Arc::new(c_persistent),
            }),
        }
    }

    /// Explicitly clone Self. This clones the contained configuration as well. This should only
    /// be done for passing off a new configuration for updating to the providers.
    pub fn explicit_clone(&self) -> Self {
        Self::new_separate(
            ConnectionConfiguration::clone(&self.session),
            ConnectionConfiguration::clone(&self.persistent),
        )
    }

    #[allow(clippy::result_large_err)]
    fn collapse_configs_if_effectively_eq(
        c1: ConnectionConfiguration,
        c2: ConnectionConfiguration,
    ) -> Result<ConnectionConfiguration, (ConnectionConfiguration, ConnectionConfiguration)> {
        if c1.config != c2.config {
            return Err((c1, c2));
        }

        if c1.provider_tag != c2.provider_tag || c1.connection_id != c2.connection_id {
            return Err((c1, c2));
        }

        if !Arc::ptr_eq(&c1.secret_manager, &c2.secret_manager) {
            return Err((c1, c2));
        }

        match (
            c1.pending_secret_changes.is_empty(),
            c2.pending_secret_changes.is_empty(),
        ) {
            (true, true) => Ok(c1),
            (false, true) => Ok(c1),
            (true, false) => Ok(c2),
            (false, false) => Err((c1, c2)),
        }
    }
}
