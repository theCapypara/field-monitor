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

use std::cell::RefCell;
use std::collections::HashMap;
use std::mem::take;
use std::sync::Arc;

use adw::subclass::prelude::*;
use futures::future::{LocalBoxFuture, try_join_all};
use gtk::glib;

use crate::secrets::SecretManager;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ConnectionConfiguration {
        pub(super) config: RefCell<HashMap<String, serde_yaml::Value>>,
        pub(super) provider_tag: RefCell<Option<String>>,
        pub(super) connection_id: RefCell<Option<String>>,
        pub(super) secret_manager: RefCell<Option<Arc<SecretManager>>>,
        pub(super) pending_secret_changes: RefCell<HashMap<String, Option<String>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionConfiguration {
        const NAME: &'static str = "ConnectionConfiguration";
        type Type = super::ConnectionConfiguration;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for ConnectionConfiguration {}
}

glib::wrapper! {
    pub struct ConnectionConfiguration(ObjectSubclass<imp::ConnectionConfiguration>);
}

static NOT_INIT: &str = "ConnectionConfiguration was not properly initialized";

impl ConnectionConfiguration {
    pub(crate) fn new(
        connection_id: String,
        provider_tag: String,
        secret_manager: Arc<SecretManager>,
    ) -> Self {
        let slf: Self = glib::Object::builder().build();
        let imp = slf.imp();
        imp.provider_tag.replace(Some(provider_tag));
        imp.connection_id.replace(Some(connection_id));
        imp.secret_manager.replace(Some(secret_manager));
        slf
    }

    pub(crate) fn tag(&self) -> String {
        self.imp().provider_tag.borrow().clone().expect(NOT_INIT)
    }

    pub(crate) fn id(&self) -> String {
        self.imp().connection_id.borrow().clone().expect(NOT_INIT)
    }

    /// Saves pending secret changes to the keychain, returns configuration.
    pub(crate) async fn save(&mut self) -> anyhow::Result<HashMap<String, serde_yaml::Value>> {
        let imp = self.imp();
        let pending_secret_changes = take(&mut *imp.pending_secret_changes.borrow_mut());
        let mut futs: Vec<LocalBoxFuture<anyhow::Result<()>>> =
            Vec::with_capacity(pending_secret_changes.len());
        for (k, v) in pending_secret_changes {
            let secret_manager = imp
                .secret_manager
                .borrow()
                .as_ref()
                .expect(NOT_INIT)
                .clone();
            let connection_id = imp.connection_id.borrow().as_ref().expect(NOT_INIT).clone();
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
        Ok(imp.config.borrow().clone())
    }

    pub fn get(&self, key: &str) -> Option<serde_yaml::Value> {
        self.imp().config.borrow().get(key).cloned()
    }
    pub fn get_try_as_str(&self, key: &str) -> Option<String> {
        self.get(key)
            .and_then(|v| v.as_str().map(ToOwned::to_owned))
    }
    pub fn get_try_as_u64(&self, key: &str) -> Option<u64> {
        self.get(key).and_then(|v| v.as_u64())
    }
    pub fn get_try_as_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_i64())
    }
    pub fn set(&mut self, key: impl ToString, value: impl Into<serde_yaml::Value>) {
        self.imp()
            .config
            .borrow_mut()
            .insert(key.to_string(), value.into());
    }
    pub async fn get_secret(&self, key: impl AsRef<str>) -> anyhow::Result<Option<String>> {
        if let Some(v) = self.imp().pending_secret_changes.borrow().get(key.as_ref()) {
            return Ok(v.clone());
        }
        self.do_get_secret(key).await
    }
    pub fn clear_secret(&mut self, key: impl ToString) {
        self.imp()
            .pending_secret_changes
            .borrow_mut()
            .insert(key.to_string(), None);
    }
    pub fn set_secret(&mut self, key: impl ToString, value: impl ToString) {
        self.imp()
            .pending_secret_changes
            .borrow_mut()
            .insert(key.to_string(), Some(value.to_string()));
    }

    async fn do_get_secret(&self, key: impl AsRef<str>) -> anyhow::Result<Option<String>> {
        let secret_manager = self
            .imp()
            .secret_manager
            .borrow()
            .as_ref()
            .expect(NOT_INIT)
            .clone();
        let connection_id = self
            .imp()
            .connection_id
            .borrow()
            .as_ref()
            .expect(NOT_INIT)
            .clone();
        secret_manager
            .lookup(&connection_id, key.as_ref())
            .await
            .map(|gstr| gstr.map(Into::into))
            .map_err(Into::into)
    }
    async fn do_clear_secret(
        secret_manager: Arc<SecretManager>,
        connection_id: String,
        key: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        secret_manager
            .clear(&connection_id, key.as_ref())
            .await
            .map_err(Into::into)
    }
    async fn do_set_secret(
        secret_manager: Arc<SecretManager>,
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
