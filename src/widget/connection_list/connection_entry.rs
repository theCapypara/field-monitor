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
use std::ops::Deref;

use adw::prelude::*;
use adw::prelude::{BinExt, PreferencesGroupExt};
use adw::subclass::prelude::*;
use futures::future::try_join_all;
use gettextrs::gettext;
use glib::object::{IsA, ObjectExt};
use gtk::glib;
use log::{error, info, warn};

use libfieldmonitor::connection::*;

use crate::application::FieldMonitorApplication;
use crate::widget::connection_list::common::{add_actions_to_entry, CanHaveSuffix};
use crate::widget::connection_list::server_entry::new_server_entry_row;

const LOAD_STATE_LOADING: &str = "loading";
const LOAD_STATE_SERVERS: &str = "servers";
const LOAD_STATE_ERROR: &str = "error";
const LOAD_STATE_AUTH_REQUIRED: &str = "auth-required";

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorCLConnectionEntry)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/connection_list/connection_entry.ui")]
    pub struct FieldMonitorCLConnectionEntry {
        #[template_child]
        pub servers: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub settings_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub auth_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub header_suffix: TemplateChild<gtk::Box>,
        #[template_child]
        pub connections_bin: TemplateChild<adw::Bin>,
        #[property(get, set)]
        pub connection: RefCell<Option<ConnectionInstance>>,
        #[property(get, set)]
        pub error_text: RefCell<String>,
        #[property(get, set, default=LOAD_STATE_SERVERS)]
        pub load_state: RefCell<String>,
        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorCLConnectionEntry {
        const NAME: &'static str = "FieldMonitorCLConnectionEntry";
        type Type = super::FieldMonitorCLConnectionEntry;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorCLConnectionEntry {}
    impl WidgetImpl for FieldMonitorCLConnectionEntry {}
    impl PreferencesGroupImpl for FieldMonitorCLConnectionEntry {}
}

glib::wrapper! {
    pub struct FieldMonitorCLConnectionEntry(ObjectSubclass<imp::FieldMonitorCLConnectionEntry>)
        @extends gtk::Widget, adw::PreferencesGroup;
}

impl FieldMonitorCLConnectionEntry {
    pub fn new(app: &FieldMonitorApplication, connection: &ConnectionInstance) -> Self {
        let metadata = connection.metadata();
        let slf: Self = glib::Object::builder()
            .property("application", app)
            .property("connection", connection)
            .property("title", metadata.title)
            .build();
        let imp = slf.imp();

        let connection_id = connection.connection_id();
        // TODO: I couldn't make this work with a binding, maybe not supported this way?
        imp.settings_button.set_action_target(Some(&connection_id));
        imp.auth_button.set_action_target(Some(&connection_id));

        slf
    }

    async fn try_update_connection(&self) -> ConnectionResult<()> {
        let connection = self.connection().unwrap();

        let metadata = connection.metadata();
        self.set_title(&metadata.title);
        if let Some(subtitle) = metadata.subtitle.as_deref() {
            self.set_description(Some(subtitle));
        }

        // Remove and re-add action buttons
        self.imp().connections_bin.set_child(Some(
            &gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(8)
                .build(),
        ));
        add_actions_to_entry(
            self,
            false,
            &connection.connection_id(),
            connection.actions(),
        );

        let servers = connection.servers().await?;

        let mut load_subservers = Vec::with_capacity(servers.len());

        if servers.is_empty() {
            self.imp().servers.append(
                &adw::ActionRow::builder()
                    .sensitive(false)
                    .title(gettext("No servers available"))
                    .build(),
            );
        }

        for (server_id, server) in servers {
            let connection_id = connection.connection_id();

            load_subservers.push(async move {
                new_server_entry_row(
                    &self.application().unwrap(),
                    connection_id,
                    vec![server_id.into_owned()],
                    server,
                )
                .await
            });
        }

        let servers = try_join_all(load_subservers.into_iter()).await?;
        for server in servers {
            self.imp().servers.append(&server);
        }

        self.set_load_state(LOAD_STATE_SERVERS);
        Ok(())
    }

    fn connection_load_error(&self, err: ConnectionError) {
        self.imp().servers.remove_all();

        match err {
            ConnectionError::AuthFailed(_, internal_err) => {
                info!(
                    "Connection {:?} auth error: {:?}",
                    self.connection()
                        .as_ref()
                        .map(ConnectionInstance::connection_id),
                    internal_err
                );
                self.set_load_state(LOAD_STATE_AUTH_REQUIRED);
            }
            ConnectionError::General(msg, internal_err) => {
                error!(
                    "Connection {:?} load error: {:?} - ({:?})",
                    self.connection()
                        .as_ref()
                        .map(ConnectionInstance::connection_id),
                    internal_err,
                    msg
                );
                self.set_error_text(msg.as_deref().unwrap_or_default());
                self.set_load_state(LOAD_STATE_ERROR);
            }
        }
    }

    pub fn server_titles(&self) -> impl IntoIterator<Item = impl Deref<Target = str>> {
        let mut result = vec![];
        for server in self.imp().servers.observe_children().iter::<glib::Object>() {
            let Ok(server) = server else {
                return result;
            };
            if let Some(server) = server.downcast_ref::<adw::PreferencesRow>() {
                result.push(server.title())
            } else {
                warn!("Invalid server row type: {}", server.type_())
            }
        }
        result
    }
}

#[gtk::template_callbacks]
impl FieldMonitorCLConnectionEntry {
    #[template_callback]
    async fn on_self_connection_changed(&self) {
        let connection = self.connection().unwrap();
        self.set_title(&connection.title());
        self.set_load_state(LOAD_STATE_LOADING);
        self.imp().servers.remove_all();

        if let Err(err) = self.try_update_connection().await {
            self.connection_load_error(err);
        }
    }
}

impl CanHaveSuffix for FieldMonitorCLConnectionEntry {
    fn add_suffix(&self, widget: &impl IsA<gtk::Widget>) {
        self.imp()
            .connections_bin
            .child()
            .unwrap()
            .downcast::<gtk::Box>()
            .unwrap()
            .prepend(widget);
    }
}
