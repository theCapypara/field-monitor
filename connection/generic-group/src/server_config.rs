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
use std::num::NonZeroU32;
use std::ops::{Deref, DerefMut};

use futures::future::LocalBoxFuture;
use glib::prelude::*;
use glib::subclass::prelude::*;
use secure_string::SecureString;

use crate::preferences::GenericGroupConfiguration;

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::ServerConfigForRow)]
    pub struct ServerConfigForRow {
        #[property(get, construct_only)]
        pub key: RefCell<String>,
        #[property(get, set)]
        pub title: RefCell<String>,
        #[property(get, set)]
        pub host: RefCell<String>,
        #[property(get, set)]
        pub port: RefCell<u32>,
        #[property(get, set, nullable)]
        pub user: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServerConfigForRow {
        const NAME: &'static str = "ServerConfigForRow";
        type Type = super::ServerConfigForRow;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServerConfigForRow {}
}

glib::wrapper! {
    pub struct ServerConfigForRow(ObjectSubclass<imp::ServerConfigForRow>);
}

#[derive(Debug, Clone)]
pub struct FinalizedServerConfig {
    pub key: String,
    pub title: String,
    pub host: String,
    pub port: NonZeroU32,
    pub user: Option<String>,
    pub password: Option<SecureString>,
    pub user_remember: bool,
    pub password_remember: bool,
}

impl Default for FinalizedServerConfig {
    fn default() -> Self {
        FinalizedServerConfig {
            key: String::default(),
            title: String::default(),
            host: String::default(),
            port: NonZeroU32::new(1).unwrap(),
            user: None,
            password: None,
            user_remember: bool::default(),
            password_remember: bool::default(),
        }
    }
}

impl FinalizedServerConfig {
    pub fn user_if_remembered(&self) -> Option<&str> {
        if self.user_remember {
            self.user.as_deref()
        } else {
            None
        }
    }

    pub fn password_if_remembered(&self) -> Option<&SecureString> {
        if self.password_remember {
            self.password.as_ref()
        } else {
            None
        }
    }
}

#[derive(Debug, Default)]
pub struct ServerConfigChanges {
    pub updates: ServerUpdateMap,
    pub removes: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct ServerUpdateMap(HashMap<String, FinalizedServerConfig>);

impl Deref for ServerUpdateMap {
    type Target = HashMap<String, FinalizedServerConfig>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ServerUpdateMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// This implementation is used when re-opening an edited server.
impl GenericGroupConfiguration for ServerUpdateMap {
    fn connection_title(&self) -> Option<&str> {
        None
    }

    fn title(&self, server: &str) -> Option<String> {
        self.0.get(server).map(|s| s.title.clone())
    }

    fn host(&self, server: &str) -> Option<String> {
        self.0.get(server).map(|s| s.host.clone())
    }

    fn port(&self, server: &str) -> Option<NonZeroU32> {
        self.0.get(server).map(|s| s.port)
    }

    fn user(&self, server: &str) -> Option<String> {
        self.0.get(server).and_then(|s| {
            if s.user_remember {
                s.user.clone()
            } else {
                None
            }
        })
    }

    fn password(&self, server: &str) -> LocalBoxFuture<anyhow::Result<Option<SecureString>>> {
        let server = server.to_string();
        Box::pin(async move {
            Ok(self.0.get(&server).and_then(|s| {
                if s.password_remember {
                    s.password.clone()
                } else {
                    None
                }
            }))
        })
    }

    fn set_connection_title(&mut self, _value: &str) {
        unimplemented!()
    }

    fn set_title(&mut self, _server: &str, _value: &str) {
        unimplemented!()
    }

    fn set_host(&mut self, _server: &str, _value: &str) {
        unimplemented!()
    }

    fn set_port(&mut self, _server: &str, _value: NonZeroU32) {
        unimplemented!()
    }

    fn set_user(&mut self, _server: &str, _value: Option<&str>) {
        unimplemented!()
    }

    fn set_password(&mut self, _server: &str, _value: Option<SecureString>) {
        unimplemented!()
    }

    fn set_password_session(&mut self, _server: &str, _value: Option<&SecureString>) {
        unimplemented!()
    }

    fn remove_server(&mut self, _server: &str) {
        unimplemented!()
    }
}

impl ServerUpdateMap {
    /// Returns a struct that implements GenericGroupConfiguration and either
    /// returns values from Self, or the passed in group configuration if not found.
    /// Write operations panic on the returned object.
    pub fn either_or<T: GenericGroupConfiguration>(self, other: T) -> EitherOrConfigMap<Self, T> {
        EitherOrConfigMap(self, other)
    }
}

#[derive(Debug)]
pub struct EitherOrConfigMap<A, B>(A, B);

impl<A, B> GenericGroupConfiguration for EitherOrConfigMap<A, B>
where
    A: GenericGroupConfiguration,
    B: GenericGroupConfiguration,
{
    fn connection_title(&self) -> Option<&str> {
        self.0.connection_title().or(self.1.connection_title())
    }

    fn title(&self, server: &str) -> Option<String> {
        self.0.title(server).or(self.1.title(server))
    }

    fn host(&self, server: &str) -> Option<String> {
        self.0.host(server).or(self.1.host(server))
    }

    fn port(&self, server: &str) -> Option<NonZeroU32> {
        self.0.port(server).or(self.1.port(server))
    }

    fn user(&self, server: &str) -> Option<String> {
        self.0.user(server).or(self.1.user(server))
    }

    fn password(&self, server: &str) -> LocalBoxFuture<anyhow::Result<Option<SecureString>>> {
        let server = server.to_string();
        Box::pin(async move {
            let a_opt = self.0.password(&server).await?;
            match a_opt {
                Some(a) => Ok(Some(a)),
                None => self.1.password(&server).await,
            }
        })
    }

    fn set_connection_title(&mut self, _value: &str) {
        unimplemented!()
    }

    fn set_title(&mut self, _server: &str, _value: &str) {
        unimplemented!()
    }

    fn set_host(&mut self, _server: &str, _value: &str) {
        unimplemented!()
    }

    fn set_port(&mut self, _server: &str, _value: NonZeroU32) {
        unimplemented!()
    }

    fn set_user(&mut self, _server: &str, _value: Option<&str>) {
        unimplemented!()
    }

    fn set_password(&mut self, _server: &str, _value: Option<SecureString>) {
        unimplemented!()
    }

    fn set_password_session(&mut self, _server: &str, _value: Option<&SecureString>) {
        unimplemented!()
    }

    fn remove_server(&mut self, _server: &str) {
        unimplemented!()
    }
}
