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
pub const DEFAULT_GENERIC_ICON: &str = "network-server-symbolic";

mod connection_list_navbar;
mod connection_stack;
mod info_page;
mod server_group;
mod server_row;

use adw::prelude::*;
pub use connection_list_navbar::*;
pub use connection_stack::*;
use gettextrs::gettext;
use glib::object::Cast;
use gtk::gio;
use libfieldmonitor::connection::*;
use libfieldmonitor::i18n::gettext_f;
use oo7::zbus::export::futures_util::future::join;
use std::borrow::Cow;

async fn make_server_prefix_suffix(
    server: &dyn ServerConnection,
    path: &[String],
    row: Option<&impl IsA<adw::ActionRow>>,
) -> ConnectionResult<(gtk::Widget, gtk::Widget)> {
    let path = path.join("/");
    let metadata = server.metadata().await;

    let prefix = make_icon(&metadata);

    let suffix = gtk::Box::builder()
        .spacing(6)
        .orientation(gtk::Orientation::Horizontal)
        .build();
    join(
        maybe_add_connect_button(row, &suffix, server, &path),
        maybe_add_actions_button(&suffix, ServerOrConnection::Server(server), &path),
    )
    .await;

    Ok((prefix, suffix.upcast()))
}

fn make_icon(metadata: &ServerMetadata) -> gtk::Widget {
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
        IconSpec::Custom(factory) => factory(metadata),
    };

    add_status(wdg, metadata)
}

fn add_status(child_wdgt: gtk::Widget, metadata: &ServerMetadata) -> gtk::Widget {
    let parent = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .valign(gtk::Align::Center)
        .spacing(6)
        .build();

    parent.append(&child_wdgt);

    match metadata.is_online {
        Some(status) => {
            let (class, tooltip_text, icon_name) = if status {
                ("success", "Online", "circle-filled-symbolic")
            } else {
                ("dim-label", "Offline", "circle-outline-thick-symbolic")
            };

            let status_icon = gtk::Image::builder()
                .pixel_size(8)
                .icon_name(icon_name)
                .css_classes([class])
                .tooltip_text(tooltip_text)
                .valign(gtk::Align::Center)
                .build();

            parent.append(&status_icon);
        }
        None => {
            let placeholder = gtk::Box::builder().width_request(8).build();

            parent.append(&placeholder);
        }
    }

    parent.upcast()
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

async fn maybe_add_actions_button(
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
            "Connect via {adapter}",
            &[("adapter", &adapter_label)],
        ))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
        .upcast()
}

enum ServerOrConnection<'a> {
    Server(&'a dyn ServerConnection),
    Connection(&'a dyn Connection),
}
