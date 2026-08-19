/* Copyright 2024-2026 Marco Köpcke
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
use crate::widget::connection_list::ServerOrConnection;
use crate::widget::connection_list::server_info::ServerInfoWidget;
use adw::gio;
use adw::prelude::{ActionRowExt, BinExt};
use gettextrs::gettext;
use glib::{Object, WeakRef};
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use libfieldmonitor::connection::*;
use libfieldmonitor::i18n::gettext_f;
use log::debug;
use std::borrow::Cow;
use std::cell::Cell;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone, Copy, Debug)]
enum ButtonSlot {
    Edit = 0,
    Connect = 1,
    Actions = 2,
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct FieldMonitorServerActions {
        pub edit_button: WeakRef<gtk::Widget>,
        pub connect_button: WeakRef<gtk::Widget>,
        pub actions_button: WeakRef<gtk::Widget>,
        pub last_state_editable: Cell<bool>,
        // length, hash of the vec
        pub last_state_adapters: Cell<(usize, u64)>,
        // length, hash of the vec
        pub last_state_actions: Cell<(usize, u64)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorServerActions {
        const NAME: &'static str = "FieldMonitorServerActions";
        type Type = super::FieldMonitorServerActions;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for FieldMonitorServerActions {}
    impl WidgetImpl for FieldMonitorServerActions {}
    impl BoxImpl for FieldMonitorServerActions {}
}

glib::wrapper! {
    pub struct FieldMonitorServerActions(ObjectSubclass<imp::FieldMonitorServerActions>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl FieldMonitorServerActions {
    /// Updates the server connect/edit/actions buttons.
    /// This will add a FieldMonitorServerActions to the widget if it doesn't already have one.
    pub async fn update_server_info_widget(
        widget: &impl ServerInfoWidget,
        server: &dyn ServerConnection,
        path: &str,
    ) {
        let act_bin = widget.get_actions_container();
        let slf = act_bin.child().and_downcast::<Self>().unwrap_or_else(|| {
            let slf = Self::new();
            act_bin.set_child(Some(&slf));
            slf
        });

        slf.process_edit_button(server, path).await;
        slf.process_connect_button(widget.get_row(), server, path)
            .await;
        slf.process_action_buttons(ServerOrConnection::Server(server), path)
            .await;
    }

    /// Create an instance for a connection to be placed on a connection info page (for
    /// connection actions).
    pub async fn new_for_connection(connection: &ConnectionInstance) -> Self {
        let slf = Self::new();
        slf.process_action_buttons(
            ServerOrConnection::Connection(connection),
            &connection.connection_id(),
        )
        .await;
        slf
    }

    fn new() -> Self {
        Object::builder()
            .property("spacing", 6)
            .property("orientation", gtk::Orientation::Horizontal)
            .build()
    }

    /// Adds, updates, or removes the edit button if `editable` has changed since
    /// the last time this was called.
    async fn process_edit_button(&self, server: &dyn ServerConnection, path: &str) {
        let imp = self.imp();
        let last_state_editable = imp.last_state_editable.get();
        let current_editable = server.editable();
        imp.last_state_editable.set(current_editable);
        if !last_state_editable && current_editable {
            debug!("{path}: adding edit button for server");
            let label = gettext("Edit Server");
            let button = gtk::Button::builder()
                .action_name("app.edit-connection-server")
                .action_target(&path.to_variant())
                .icon_name("edit-symbolic")
                .tooltip_text(&label)
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .build();
            button.update_property(&[gtk::accessible::Property::Label(&label)]);

            self.set_slot(ButtonSlot::Edit, Some(button.upcast_ref()));
        } else if last_state_editable && !current_editable {
            debug!("{path}: removing edit button for server");
            self.set_slot(ButtonSlot::Edit, None);
        }
    }

    /// Adds, updates, or removes the connect button if the supported adapters have changed since
    /// the last time this was called.
    async fn process_connect_button(
        &self,
        row: Option<&impl IsA<adw::ActionRow>>,
        server: &dyn ServerConnection,
        path: &str,
    ) {
        let imp = self.imp();
        let (last_len, last_hash) = imp.last_state_adapters.get();
        let adapters = server.supported_adapters().await;
        let new_len = adapters.len();
        if new_len == 0 && last_len == new_len {
            return;
        }
        let new_hash = hash(&adapters);
        if last_hash == new_hash {
            return;
        }
        imp.last_state_adapters.set((new_len, new_hash));
        debug!("{path}: updating connect button for server");

        let connect_button = if adapters.len() == 1 {
            let adapter = adapters.into_iter().next().unwrap();
            Some(make_single_connect_button(path, adapter))
        } else if !adapters.is_empty() {
            Some(make_multi_connection_button(path, adapters))
        } else {
            None
        };

        if let Some(row) = row {
            let row = row.upcast_ref();
            row.set_activatable_widget(connect_button.as_ref());
            row.set_activatable(connect_button.is_some());
        }

        self.set_slot(ButtonSlot::Connect, connect_button.as_ref());
    }

    /// Adds, updates, or removes the actions button if the supported actions have changed since
    /// the last time this was called.
    pub async fn process_action_buttons(
        &self,
        server_or_connection: ServerOrConnection<'_>,
        path: &str,
    ) {
        let imp = self.imp();
        let (last_len, last_hash) = imp.last_state_actions.get();
        let (actions, is_server) = match server_or_connection {
            ServerOrConnection::Server(server) => (server.actions().await, true),
            ServerOrConnection::Connection(connection) => (connection.actions().await, false),
        };
        let new_len = actions.len();
        if new_len == 0 && last_len == new_len {
            return;
        }
        let new_hash = hash(&actions);
        if last_hash == new_hash {
            return;
        }
        imp.last_state_actions.set((new_len, new_hash));
        debug!("{path}: updating action button for connection/server");

        if actions.is_empty() {
            self.set_slot(ButtonSlot::Actions, None);
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

        let label = gettext("Actions");
        let button = gtk::MenuButton::builder()
            .menu_model(&menu)
            .icon_name("view-more-symbolic")
            .tooltip_text(&label)
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        button.update_property(&[gtk::accessible::Property::Label(&label)]);

        self.set_slot(ButtonSlot::Actions, Some(button.upcast_ref()));
    }

    fn set_slot(&self, pos: ButtonSlot, new_value: Option<&gtk::Widget>) {
        let imp = self.imp();
        let slot = match pos {
            ButtonSlot::Edit => &imp.edit_button,
            ButtonSlot::Connect => &imp.connect_button,
            ButtonSlot::Actions => &imp.actions_button,
        };
        match (new_value, slot.upgrade()) {
            (Some(v), None) => {
                insert_into_box(self.upcast_ref(), self.occupied_slots_before(pos), v);
                slot.set(Some(v));
            }
            (Some(v), Some(old)) => {
                replace_in_box(self.upcast_ref(), &old, Some(v));
                slot.set(Some(v));
            }
            (None, Some(old)) => {
                replace_in_box(self.upcast_ref(), &old, None);
                slot.set(None);
            }
            (None, None) => {}
        }
    }

    /// Number of slots that come before `pos` in display order and currently have a widget
    /// present in the box.
    fn occupied_slots_before(&self, pos: ButtonSlot) -> usize {
        let imp = self.imp();
        [
            (ButtonSlot::Edit, &imp.edit_button),
            (ButtonSlot::Connect, &imp.connect_button),
            (ButtonSlot::Actions, &imp.actions_button),
        ]
        .into_iter()
        .filter(|(slot, weak)| (*slot as usize) < pos as usize && weak.upgrade().is_some())
        .count()
    }
}

/// Insert a widget at the given index in the box. If the index is equal to or bigger than the
/// number of children in the box, it will be inserted at the end.
fn insert_into_box(boxx: &gtk::Box, index: usize, widget: &gtk::Widget) {
    let sibling = if index == 0 {
        None
    } else {
        nth_child(boxx, index - 1).or_else(|| boxx.last_child())
    };
    boxx.insert_child_after(widget, sibling.as_ref());
}

/// Replace `old` in `boxx` with `new`. If `new` is `None` `old` is just removed.
fn replace_in_box(boxx: &gtk::Box, old: &gtk::Widget, new: Option<&gtk::Widget>) {
    if let Some(new) = new {
        boxx.insert_child_after(new, old.prev_sibling().as_ref());
    }
    boxx.remove(old);
}

/// Returns the `n`-th (0-based) child of `boxx`, if it has that many children.
fn nth_child(boxx: &gtk::Box, n: usize) -> Option<gtk::Widget> {
    let mut child = boxx.first_child();
    let mut i = 0;
    while let Some(c) = child {
        if i == n {
            return Some(c);
        }
        child = c.next_sibling();
        i += 1;
    }
    None
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

    let label = gettext("Connect");
    let button = gtk::MenuButton::builder()
        .menu_model(&menu)
        .icon_name("display-with-window-symbolic")
        .tooltip_text(&label)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    button.update_property(&[gtk::accessible::Property::Label(&label)]);
    button.upcast()
}

fn make_single_connect_button(
    path: &str,
    (adapter_id, adapter_label): (Cow<str>, Cow<str>),
) -> gtk::Widget {
    let label = gettext_f(
        // Translators: Do NOT translate the content between '{' and '}', this is a
        // variable name.
        "Connect via {adapter}",
        &[("adapter", &adapter_label)],
    );
    let button = gtk::Button::builder()
        .action_name("app.connect-to-server")
        .action_target(&(path, &*adapter_id).to_variant())
        .icon_name("display-with-window-symbolic")
        .tooltip_text(&label)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    button.update_property(&[gtk::accessible::Property::Label(&label)]);
    button.upcast()
}

fn hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
