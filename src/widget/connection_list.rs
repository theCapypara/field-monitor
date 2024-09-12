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
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::time::Duration;

use adw::prelude::*;
use adw::subclass::prelude::*;
use futures::StreamExt;
use glib::clone;
use glib::object::ObjectExt;
use gtk::gio;
use gtk::glib;
use log::{debug, warn};
use lru::LruCache;

use libfieldmonitor::connection::ConnectionInstance;

use crate::application::FieldMonitorApplication;
use crate::widget::connection_list::connection_entry::FieldMonitorCLConnectionEntry;

mod connection_entry;
mod server_entry;

mod imp {
    use lru::LruCache;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorConnectionList)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/connection_list.ui")]
    pub struct FieldMonitorConnectionList {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub connection_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub connection_list_model: TemplateChild<gio::ListStore>,
        #[template_child]
        pub connection_list_model_sorted: TemplateChild<gtk::SortListModel>,
        #[template_child]
        pub connection_list_model_sorted_filtered: TemplateChild<gtk::FilterListModel>,
        #[template_child]
        pub model_string_filter: TemplateChild<gtk::StringFilter>,
        #[template_child]
        pub empty_list_status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub welcome_status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        pub connections: RefCell<Option<HashMap<String, ConnectionInstance>>>,
        pub cached_list_box_rows: RefCell<Option<LruCache<ConnectionInstance, gtk::ListBoxRow>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorConnectionList {
        const NAME: &'static str = "FieldMonitorConnectionList";
        type Type = super::FieldMonitorConnectionList;
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
    impl ObjectImpl for FieldMonitorConnectionList {}
    impl WidgetImpl for FieldMonitorConnectionList {}
    impl BinImpl for FieldMonitorConnectionList {}
}

glib::wrapper! {
    pub struct FieldMonitorConnectionList(ObjectSubclass<imp::FieldMonitorConnectionList>)
        @extends gtk::Widget, adw::Bin;
}

impl FieldMonitorConnectionList {
    pub fn new(app: &FieldMonitorApplication) -> Self {
        let slf: Self = glib::Object::builder().property("application", app).build();
        let imp = slf.imp();
        imp.connections.replace(Some(HashMap::new()));
        imp.cached_list_box_rows
            .replace(Some(LruCache::new(NonZeroUsize::new(50).unwrap())));
        // Fill list, if empty and loading connections, show welcome page, otherwise show empty
        let connections = app.connections().into_iter().collect::<Vec<_>>();
        if connections.is_empty() {
            if app.loading_connections() {
                imp.stack.set_visible_child_name("empty_list");
            } else {
                imp.stack.set_visible_child_name("welcome");
            }
        } else {
            for connection in connections {
                slf.on_update_connection(connection);
            }
        }
        if app.loading_connections() {
            slf.show_loading_connections();
        }
        app.connect_notify_local(
            Some("loading-connections"),
            glib::clone!(
                #[weak]
                slf,
                move |app, _| {
                    if app.loading_connections() {
                        slf.show_loading_connections()
                    }
                }
            ),
        );
        app.connect_closure(
            "connection-updated",
            false,
            glib::closure_local!(
                #[watch]
                slf,
                move |_: FieldMonitorApplication, instance: ConnectionInstance| {
                    slf.on_update_connection(instance);
                }
            ),
        );
        app.connect_closure(
            "connection-removed",
            false,
            glib::closure_local!(
                #[watch]
                slf,
                move |_: FieldMonitorApplication, id: String| {
                    slf.on_connection_removed(&id);
                }
            ),
        );

        let property_expr = gtk::PropertyExpression::new(
            ConnectionInstance::static_type(),
            None::<&gtk::Expression>,
            "title",
        );
        imp.connection_list_model_sorted
            .set_sorter(Some(&gtk::StringSorter::new(Some(&property_expr))));
        imp.model_string_filter.set_expression(Some(&property_expr));
        imp.connection_list_model_sorted_filtered
            .set_filter(Some(&*slf.imp().model_string_filter));
        imp.connection_list_box.bind_model(
            Some(&*slf.imp().connection_list_model_sorted_filtered),
            clone!(
                #[weak_allow_none]
                slf,
                move |obj| {
                    match slf {
                        Some(slf) => {
                            let connection_instance: &ConnectionInstance =
                                obj.downcast_ref().unwrap();
                            let mut cache_borrow = slf.imp().cached_list_box_rows.borrow_mut();
                            let cache_ref = cache_borrow.as_mut().unwrap();
                            cache_ref
                                .get_or_insert(connection_instance.clone(), || {
                                    debug!("new row");
                                    let wdg = FieldMonitorCLConnectionEntry::new(
                                        slf.application().as_ref().unwrap(),
                                        connection_instance,
                                    );
                                    gtk::ListBoxRow::builder()
                                        .child(&wdg)
                                        .activatable(false)
                                        .selectable(true)
                                        .build()
                                })
                                .clone()
                        }
                        .upcast(),
                        None => {
                            warn!("Tried to add a widget while list was unloaded");
                            gtk::Box::default().upcast()
                        }
                    }
                }
            ),
        );

        if let Some(app_id) = app.application_id() {
            let icon_name = format!("{}-symbolic", app_id);
            imp.empty_list_status_page.set_icon_name(Some(&icon_name));
            imp.welcome_status_page.set_icon_name(Some(&icon_name));
        }

        slf
    }

    pub fn toggle_search(&self) {
        self.imp()
            .search_bar
            .set_search_mode(!self.imp().search_bar.is_search_mode())
    }

    fn show_loading_connections(&self) {
        // do not recurse:
        if self.imp().progress_bar.get_visible() {
            return;
        }
        if self.has_no_connections() {
            self.imp().stack.set_visible_child_name("welcome");
        }
        let slf_weak = self.downgrade();
        self.imp().progress_bar.set_visible(true);
        glib::spawn_future_local(async move {
            while glib::interval_stream(Duration::from_millis(100))
                .next()
                .await
                .is_some()
            {
                if let Some(slf) = slf_weak.upgrade() {
                    let app_brw = slf.imp().application.borrow();
                    if !app_brw
                        .as_ref()
                        .map(|app| app.loading_connections())
                        .unwrap_or_default()
                    {
                        // it has finished
                        slf.imp().progress_bar.set_visible(false);
                        if slf.has_no_connections() {
                            slf.imp().stack.set_visible_child_name("empty_list");
                        }
                        break;
                    }
                    slf.imp().progress_bar.pulse();
                } else {
                    break;
                }
            }
        });
    }

    fn has_no_connections(&self) -> bool {
        self.imp()
            .connections
            .borrow()
            .as_ref()
            .map(HashMap::is_empty)
            .unwrap_or(true)
    }

    fn on_update_connection(&self, connection: ConnectionInstance) {
        let imp = self.imp();
        let mut list_brw = self.imp().connections.borrow_mut();
        let list = list_brw.as_mut().unwrap();

        let id = connection.connection_id();

        match list.entry(id) {
            Entry::Occupied(mut entry) => {
                Self::remove_from_connection_list(entry.get(), imp);
                entry.insert(connection.clone());
            }
            Entry::Vacant(entry) => {
                entry.insert(connection.clone());
            }
        };
        self.imp().stack.set_visible_child_name("list");
        imp.connection_list_model.append(&connection);
    }

    fn on_connection_removed(&self, id: &str) {
        let imp = self.imp();
        let mut list_brw = self.imp().connections.borrow_mut();
        let list = list_brw.as_mut().unwrap();
        if let Entry::Occupied(entry) = list.entry(id.to_string()) {
            Self::remove_from_connection_list(entry.get(), imp);
            entry.remove();
        }
        if list.is_empty() {
            self.imp().stack.set_visible_child_name("empty_list");
        }
    }

    fn remove_from_connection_list(
        connection: &ConnectionInstance,
        imp: &imp::FieldMonitorConnectionList,
    ) {
        if let Some(pos) = imp.connection_list_model.find(connection) {
            imp.connection_list_model.remove(pos);
        }
    }
}

#[gtk::template_callbacks]
impl FieldMonitorConnectionList {
    #[template_callback]
    fn on_search_entry_search_changed(&self) {
        self.imp()
            .model_string_filter
            .set_search(Some(&*self.imp().search_entry.text()));
    }
}
