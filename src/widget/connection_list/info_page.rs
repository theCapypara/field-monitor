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
use crate::application::FieldMonitorApplication;
use crate::widget::connection_list::server_group::FieldMonitorServerGroup;
use crate::widget::connection_list::server_info::maybe_add_actions_button;
use crate::widget::connection_list::server_row::FieldMonitorServerRow;
use crate::widget::connection_list::ServerOrConnection;
use adw::prelude::*;
use adw::subclass::prelude::*;
use async_std::task::block_on;
use futures::lock::Mutex;
use gettextrs::gettext;
use gtk::glib;
use libfieldmonitor::connection::*;
use log::{debug, warn};
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorConnectionInfoPage)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/connection_list/info_page.ui")]
    pub struct FieldMonitorConnectionInfoPage {
        #[template_child]
        pub status_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub settings_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub auth_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub status_page_error: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub group_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub box_for_connection_action: TemplateChild<gtk::Box>,
        #[property(get, set)]
        pub connection: RefCell<Option<ConnectionInstance>>,
        #[property(get, set)]
        pub error_text: RefCell<String>,
        #[property(get, set, default = "servers")]
        pub load_state: RefCell<String>,
        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        pub reload_connections_reentry_lock: Mutex<()>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorConnectionInfoPage {
        const NAME: &'static str = "FieldMonitorConnectionInfoPage";
        type Type = super::FieldMonitorConnectionInfoPage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorConnectionInfoPage {}
    impl WidgetImpl for FieldMonitorConnectionInfoPage {}
    impl BinImpl for FieldMonitorConnectionInfoPage {}
}

glib::wrapper! {
    pub struct FieldMonitorConnectionInfoPage(ObjectSubclass<imp::FieldMonitorConnectionInfoPage>)
        @extends gtk::Widget, adw::Bin;
}

impl FieldMonitorConnectionInfoPage {
    pub fn new(app: &FieldMonitorApplication, connection: &ConnectionInstance) -> Self {
        let slf: Self = glib::Object::builder()
            .property("application", app)
            .property("connection", connection)
            .build();
        let imp = slf.imp();

        let connection_id = connection.connection_id();
        // TODO: I couldn't make this work with a binding, maybe not supported this way?
        imp.settings_button.set_action_target(Some(&connection_id));
        imp.auth_button.set_action_target(Some(&connection_id));

        block_on(maybe_add_actions_button(
            &slf.imp().box_for_connection_action,
            ServerOrConnection::Connection(connection),
            &connection_id,
        ));

        slf
    }

    async fn reload_connection(&self) {
        if let Err(err) = self.try_reload_connection().await {
            self.error(&err);
        }
    }

    async fn try_reload_connection(&self) -> ConnectionResult<()> {
        let imp = self.imp();
        let _ = imp.reload_connections_reentry_lock.lock().await;
        imp.status_stack.set_visible_child_name("loading");
        let connection = imp.connection.borrow().clone().unwrap();
        let connection_id = connection.connection_id();

        debug!("reloading connection, removing old entries");
        while let Some(child) = imp.group_box.last_child() {
            imp.group_box.remove(&child);
        }

        let servers = connection.servers().await?;
        let no_servers = servers.is_empty();
        debug!("loaded servers");

        let mut servers_with_no_children = Vec::with_capacity(servers.len());
        let mut servers_with_children = Vec::with_capacity(servers.len());

        for (key, server) in servers {
            let server = Rc::new(server);
            let subservers = server.servers().await?;
            if subservers.is_empty() {
                servers_with_no_children.push(Server {
                    key,
                    server,
                    subservers,
                })
            } else {
                servers_with_children.push(Server {
                    key,
                    server,
                    subservers,
                })
            }
        }
        let has_servers_with_no_children = !servers_with_no_children.is_empty();
        debug!("loaded subservers");

        // Main group (servers with no children)
        let group = FieldMonitorServerGroup::new(&self.application().unwrap(), None).await?;
        for server in servers_with_no_children {
            group.add(
                &FieldMonitorServerRow::new(
                    &[connection_id.clone(), server.key.to_string()],
                    server.server,
                )
                .await?,
            );
        }
        // if servers is empty, we have no server at all, add a small note.
        if no_servers {
            group.add(
                &adw::ActionRow::builder()
                    .sensitive(false)
                    .title(gettext("No servers available"))
                    .build(),
            );
            imp.group_box.append(&group);
        } else if has_servers_with_no_children {
            imp.group_box.append(&group);
        }
        debug!("created main group");

        // Additional groups: servers with subservers
        for server in servers_with_children {
            let group = FieldMonitorServerGroup::new(
                &self.application().unwrap(),
                Some((
                    server.server.clone(),
                    &[connection_id.clone(), server.key.to_string()],
                )),
            )
            .await?;
            for (key, subserver) in server.subservers {
                group.add(
                    &FieldMonitorServerRow::new(
                        &[
                            connection_id.clone(),
                            server.key.to_string(),
                            key.to_string(),
                        ],
                        Rc::new(subserver),
                    )
                    .await?,
                );
            }
            imp.group_box.append(&group);
        }

        debug!("finished loading");
        imp.status_stack.set_visible_child_name("servers");

        Ok(())
    }

    fn error(&self, err: &ConnectionError) {
        let imp = self.imp();

        match err {
            ConnectionError::General(expl, err) => {
                imp.status_stack.set_visible_child_name("error");

                imp.status_page_error.set_description(expl.as_deref());

                warn!("failed to load connection in info page: {:?}", err);
            }
            ConnectionError::AuthFailed(_, err) => {
                imp.status_stack.set_visible_child_name("auth-required");

                debug!("failed to load connection in info page (auth): {:?}", err);
            }
        }
    }
}

#[gtk::template_callbacks]
impl FieldMonitorConnectionInfoPage {
    #[template_callback]
    async fn on_self_connection_changed(&self) {
        self.reload_connection().await;
    }
}

struct Server {
    key: Cow<'static, str>,
    server: Rc<Box<dyn ServerConnection>>,
    subservers: ServerMap,
}
