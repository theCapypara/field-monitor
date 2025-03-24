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
//! Functionality to periodically update and display information about a server (either in a
//! row or the title server of a group)

mod icon;

pub use self::icon::ServerInfoIcon;
use crate::widget::connection_list::server_info::icon::IsOnline;
use crate::widget::connection_list::{DEFAULT_GENERIC_ICON, ServerOrConnection};
use adw::prelude::*;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use glib::object::{Cast, IsA, ObjectType};
use glib::{ControlFlow, WeakRef, timeout_future};
use gtk::gio;
use libfieldmonitor::connection::{IconSpec, ServerConnection, ServerMetadata};
use libfieldmonitor::i18n::gettext_f;
use std::borrow::Cow;
use std::rc::Rc;
use std::time::Duration;

pub struct ServerInfoUpdater<T>
where
    T: ObjectType,
{
    target: WeakRef<T>,
    server: Rc<Box<dyn ServerConnection>>,
    full_path: Rc<Vec<String>>,
}

impl<T> ServerInfoUpdater<T>
where
    T: ServerInfoWidget + ObjectType,
{
    pub fn start(target: WeakRef<T>, server: Rc<Box<dyn ServerConnection>>, full_path: &[String]) {
        let full_path = Rc::new(full_path.to_vec());
        let slf = ServerInfoUpdater {
            target,
            server,
            full_path,
        };

        slf.run_updater(Self::update_metadata, Duration::from_secs(3));
        slf.run_updater(Self::update_actions, Duration::from_secs(45));
    }

    fn update_metadata(
        target: T,
        server: Rc<Box<dyn ServerConnection>>,
        path: Rc<Vec<String>>,
    ) -> LocalBoxFuture<'static, ()> {
        Box::pin(async move {
            let metadata = server.metadata().await;
            target.set_server_title(&metadata.title);
            target.set_server_subtitle(metadata.subtitle.as_deref());

            let container = target.get_icon_container();
            let new_status = match metadata.is_online {
                Some(true) => IsOnline::Online,
                Some(false) => IsOnline::Offline,
                None => IsOnline::Unknown,
            };

            // If the online status changed, we should also update the actions
            let old_status = container.status();
            if old_status != IsOnline::Unknown
                && new_status != IsOnline::Unknown
                && old_status != new_status
            {
                glib::spawn_future_local(Self::update_actions(target.clone(), server, path));
            }

            Self::update_icon(&target, new_status, metadata);
        })
    }

    fn update_actions(
        target: T,
        server: Rc<Box<dyn ServerConnection>>,
        path: Rc<Vec<String>>,
    ) -> LocalBoxFuture<'static, ()> {
        Box::pin(async move {
            let path = path.join("/");

            let suffix = gtk::Box::builder()
                .spacing(6)
                .orientation(gtk::Orientation::Horizontal)
                .build();

            Self::maybe_add_connect_button(target.get_row(), &suffix, &**server, &path).await;
            maybe_add_actions_button(&suffix, ServerOrConnection::Server(&**server), &path).await;

            target.get_actions_container().set_child(Some(&suffix));
        })
    }

    /// Run the update function in a regular interval until the target widget stops to exist.
    fn run_updater<F>(&self, cb: F, duration: Duration)
    where
        F: Fn(T, Rc<Box<dyn ServerConnection>>, Rc<Vec<String>>) -> LocalBoxFuture<'static, ()>
            + 'static,
    {
        let mut flow = ControlFlow::Continue;
        let target = self.target.clone();
        let server = self.server.clone();
        let full_path = self.full_path.clone();

        glib::spawn_future_local(async move {
            while flow == ControlFlow::Continue {
                match target.upgrade() {
                    None => {
                        flow = ControlFlow::Break;
                    }
                    Some(target) => {
                        cb(target, server.clone(), full_path.clone()).await;
                        timeout_future(duration).await;
                    }
                }
            }
        });
    }

    fn update_icon(target: &T, status: IsOnline, metadata: ServerMetadata) {
        let container = target.get_icon_container();
        container.set_status(status);

        // If true, we were able to just set an icon name in the existing widget, if false
        // we have to replace the child
        let mut simple_update = false;

        if let Some(image) = container.child().and_downcast::<gtk::Image>() {
            let icon_name = match &metadata.icon {
                IconSpec::Default => Some(DEFAULT_GENERIC_ICON),
                IconSpec::Named(name) => Some(name.as_ref()),
                _ => None,
            };
            if let Some(icon_name) = icon_name {
                simple_update = true;
                image.set_icon_name(Some(icon_name));
            };
        }

        if !simple_update {
            let wdg = match &metadata.icon {
                IconSpec::Default => gtk::Image::builder()
                    .icon_name(DEFAULT_GENERIC_ICON)
                    .build()
                    .upcast(),
                IconSpec::None => gtk::Box::builder().width_request(16).build().upcast(),
                IconSpec::Named(name) => gtk::Image::builder()
                    .icon_name(name.as_ref())
                    .build()
                    .upcast(),
                IconSpec::Custom(factory) => factory(&metadata),
            };

            container.set_child(wdg);
        }
    }

    async fn maybe_add_connect_button(
        row: Option<&impl IsA<adw::ActionRow>>,
        boxx: &gtk::Box,
        server: &dyn ServerConnection,
        path: &str,
    ) {
        let adapters = server.supported_adapters().await;

        let connect_button = if adapters.len() == 1 {
            let adapter = adapters.into_iter().next().unwrap();
            Some(make_single_connect_button(path, adapter))
        } else if !adapters.is_empty() {
            Some(make_multi_connection_button(path, adapters))
        } else {
            None
        };

        if let Some(button) = connect_button {
            if let Some(row) = row {
                row.set_activatable_widget(Some(&button));
            }
            boxx.append(&button);
        }
    }
}

pub trait ServerInfoWidget {
    fn set_server_title(&self, title: &str);
    fn set_server_subtitle(&self, subtitle: Option<&str>);
    fn get_icon_container(&self) -> ServerInfoIcon;
    fn get_actions_container(&self) -> adw::Bin;
    fn get_row(&self) -> Option<&impl IsA<adw::ActionRow>>;
}

pub async fn maybe_add_actions_button(
    boxx: &gtk::Box,
    server_or_connection: ServerOrConnection<'_>,
    path: &str,
) {
    let (actions, is_server) = match server_or_connection {
        ServerOrConnection::Server(server) => (server.actions().await, true),
        ServerOrConnection::Connection(connection) => (connection.actions().await, false),
    };

    if actions.is_empty() {
        return;
    }
    let menu = gio::Menu::new();
    for (action_id, action_title) in actions {
        let action_target = (is_server, path, &*action_id).to_variant();
        menu.append(
            Some(&*action_title),
            Some(
                gio::Action::print_detailed_name(
                    "app.perform-connection-action",
                    Some(&action_target),
                )
                .as_str(),
            ),
        );
    }

    let button = gtk::MenuButton::builder()
        .menu_model(&menu)
        .icon_name("view-more-symbolic")
        .tooltip_text(gettext("Actions"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();

    boxx.append(&button);
}

fn make_multi_connection_button(path: &str, adapters: Vec<(Cow<str>, Cow<str>)>) -> gtk::Widget {
    let menu = gio::Menu::new();
    for (adapter_id, adapter_label) in adapters {
        let action_target = (path, &*adapter_id).to_variant();
        menu.append(
            Some(&*adapter_label),
            Some(
                gio::Action::print_detailed_name("app.connect-to-server", Some(&action_target))
                    .as_str(),
            ),
        );
    }

    gtk::MenuButton::builder()
        .menu_model(&menu)
        .icon_name("display-with-window-symbolic")
        .tooltip_text(gettext("Connect"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
        .upcast()
}

fn make_single_connect_button(
    path: &str,
    (adapter_id, adapter_label): (Cow<str>, Cow<str>),
) -> gtk::Widget {
    gtk::Button::builder()
        .action_name("app.connect-to-server")
        .action_target(&(path, &*adapter_id).to_variant())
        .icon_name("display-with-window-symbolic")
        .tooltip_text(gettext_f(
            // Translators: Do NOT translate the content between '{' and '}', this is a
            // variable name.
            "Connect via {adapter}",
            &[("adapter", &adapter_label)],
        ))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
        .upcast()
}
