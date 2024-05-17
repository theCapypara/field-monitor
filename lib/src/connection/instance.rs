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
use std::rc::Rc;
use std::sync::Arc;

use glib;
use glib::prelude::*;
use glib::subclass::prelude::*;
use log::debug;

use crate::connection::configuration::ConnectionConfiguration;
use crate::connection::types::{Connection, ConnectionProvider};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ConnectionInstance {
        pub configuration: RefCell<Option<ConnectionConfiguration>>,
        pub provider: RefCell<Option<Rc<Box<dyn ConnectionProvider>>>>,
        pub implementation: RefCell<Option<Box<dyn Connection>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionInstance {
        const NAME: &'static str = "ConnectionInstance";
        type Type = super::ConnectionInstance;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for ConnectionInstance {}
}

glib::wrapper! {
    pub struct ConnectionInstance(ObjectSubclass<imp::ConnectionInstance>);
}

static NOT_INIT: &str = "ConnectionInstance was not properly initialized";

impl ConnectionInstance {
    pub async fn new(
        configuration: ConnectionConfiguration,
        provider: Rc<Box<dyn ConnectionProvider>>,
    ) -> anyhow::Result<Self> {
        let slf: Self = glib::Object::builder().build();

        // Listen to own signals for debug purposes
        let slf_id = Arc::new(configuration.id().to_string());
        slf.connect_notify(
            Some("configuration"),
            glib::clone!(@strong slf_id => move |slf,_| {
                let brw = slf.imp().configuration.borrow();
                let (id, tag) = match brw.as_ref() {
                    Some(c) => (Some(c.id()), Some(c.tag())),
                    None => (None, None)
                };
                debug!(
                    "connection instance (orig ID: {} got new config (tag: {:?}, id: {:?}).",
                    slf_id, id, tag,
                )
            }),
        );

        let imp = slf.imp();
        imp.provider.replace(Some(provider));
        slf.set_configuration(configuration).await?;
        Ok(slf)
    }

    /// Changes the configuration and recreates the implementation.
    pub async fn set_configuration(&self, value: ConnectionConfiguration) -> anyhow::Result<()> {
        let slf_imp = self.imp();
        let provider = slf_imp.provider.borrow().as_ref().expect(NOT_INIT).clone();
        let implementation = provider.load_connection(&value).await?;
        slf_imp.configuration.replace(Some(value));
        slf_imp.implementation.replace(Some(implementation));
        Ok(())
    }

    pub fn provider_tag(&self) -> Option<String> {
        self.imp()
            .configuration
            .borrow()
            .as_ref()
            .map(|c| c.tag().to_string())
    }

    pub fn id(&self) -> Option<String> {
        self.imp()
            .configuration
            .borrow()
            .as_ref()
            .map(|c| c.id().to_string())
    }
}
