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
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use adw::gdk;
use adw::prelude::*;
use adw::subclass::prelude::*;
use futures::StreamExt;
use glib::object::ObjectExt;
use gtk::gio;
use gtk::glib;
use log::debug;
use lru::LruCache;

use libfieldmonitor::connection::{Connection, ConnectionInstance};

use crate::application::FieldMonitorApplication;
use crate::widget::connection_list::connection_entry::FieldMonitorCLConnectionEntry;

mod common;
mod connection_entry;
mod server_entry;

mod imp {
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
        pub connection_list_filter: TemplateChild<gtk::CustomFilter>,
        #[template_child]
        pub empty_list_status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub welcome_status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub button_fullscreen: TemplateChild<gtk::Button>,
        #[property(get, set)]
        pub search_is_visible: AtomicBool,
        #[property(get, set)]
        pub show_overview_button: AtomicBool,
        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        pub connections: RefCell<Option<HashMap<String, FieldMonitorCLConnectionEntry>>>,
        pub row_cache: RefCell<Option<LruCache<FieldMonitorCLConnectionEntry, gtk::ListBoxRow>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorConnectionList {
        const NAME: &'static str = "FieldMonitorConnectionList";
        type Type = super::FieldMonitorConnectionList;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);

            klass.install_property_action("connection-list.toggle-search", "search-is-visible");
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
        @extends gtk::Widget, adw::Bin,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl FieldMonitorConnectionList {
    pub fn new(app: &FieldMonitorApplication, window: Option<&impl IsA<gtk::Window>>) -> Self {
        let slf: Self = glib::Object::builder().property("application", app).build();
        let imp = slf.imp();
        imp.connections.replace(Some(HashMap::new()));
        imp.row_cache
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
                        slf.show_loading_connections();
                        slf.set_cursor(gdk::Cursor::from_name("wait", None).as_ref());
                    } else if !app.busy() {
                        slf.set_cursor(None);
                    }
                }
            ),
        );
        app.connect_notify_local(
            Some("busy"),
            glib::clone!(
                #[weak]
                slf,
                move |app, _| {
                    if app.busy() {
                        slf.set_cursor(gdk::Cursor::from_name("wait", None).as_ref());
                    } else if !app.loading_connections() {
                        slf.set_cursor(None);
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
        app.bind_property("busy", &*slf.imp().stack, "sensitive")
            .invert_boolean()
            .build();

        slf.rebuild_listbox();

        if let Some(app_id) = app.application_id() {
            let icon_name = format!("{}-symbolic", app_id);
            imp.empty_list_status_page.set_icon_name(Some(&icon_name));
            imp.welcome_status_page.set_icon_name(Some(&icon_name));
        }

        if let Some(parent) = window {
            parent.connect_notify_local(
                Some("fullscreened"),
                glib::clone!(
                    #[weak]
                    slf,
                    move |window, _| {
                        if window.is_fullscreen() {
                            slf.imp()
                                .button_fullscreen
                                .set_icon_name("arrows-pointing-inward-symbolic");
                        } else {
                            slf.imp()
                                .button_fullscreen
                                .set_icon_name("arrows-pointing-outward-symbolic");
                        }
                    }
                ),
            );
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
        let mut full_refresh_required = false;

        match list.entry(id) {
            Entry::Occupied(entry) => {
                let old_title = entry.get().title();
                let new_title = connection.metadata().title;
                if old_title != new_title {
                    // Too bad, the title changed, which means we have to reload the entire model sadly,
                    // there doesn't seem to be a great way to re-sort and filter the model and box
                    // reliably.
                    full_refresh_required = true;
                }
                entry.get().set_connection(connection.clone());
            }
            Entry::Vacant(entry) => {
                let new_entry = FieldMonitorCLConnectionEntry::new(
                    self.application().as_ref().unwrap(),
                    &connection,
                );
                imp.connection_list_model.append(&new_entry);
                entry.insert(new_entry);
            }
        };

        // XXX: This is pretty hacky overall. We have to delay this a bit, otherwise the widget may
        // fail to properly redraw. It's all pretty messy sadly.
        if full_refresh_required {
            debug!("doing full refresh");
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = slf)]
                self,
                async move {
                    slf.imp().connection_list_box.set_sensitive(false);
                    async_std::task::sleep(Duration::from_millis(50)).await;
                    slf.imp()
                        .connection_list_box
                        .bind_model(None::<&gio::ListModel>, |_| gtk::Box::default().upcast());
                    async_std::task::sleep(Duration::from_millis(50)).await;
                    slf.rebuild_listbox();
                    slf.imp().connection_list_box.set_sensitive(true);
                }
            ));
        }

        self.imp().stack.set_visible_child_name("list");
    }

    fn rebuild_listbox(&self) {
        let imp = self.imp();
        let property_expr = gtk::PropertyExpression::new(
            FieldMonitorCLConnectionEntry::static_type(),
            None::<&gtk::Expression>,
            "title",
        );
        imp.connection_list_model_sorted
            .set_sorter(Some(&gtk::StringSorter::new(Some(&property_expr))));
        // Filter by connection name and server name
        imp.connection_list_filter.set_filter_func(glib::clone!(
            #[weak(rename_to = slf)]
            self,
            #[upgrade_or_default]
            move |item| {
                let search_term = slf.imp().search_entry.text().to_lowercase();
                let search_term = search_term.trim();
                if search_term.is_empty() {
                    return true;
                }
                let item: &FieldMonitorCLConnectionEntry = item.downcast_ref().unwrap();

                if item.title().to_lowercase().contains(search_term) {
                    return true;
                }
                for server_title in item.server_titles() {
                    if server_title.to_lowercase().contains(search_term) {
                        return true;
                    }
                }
                false
            }
        ));
        imp.connection_list_model_sorted_filtered
            .set_filter(Some(&*imp.connection_list_filter));
        imp.connection_list_box.bind_model(
            Some(&*imp.connection_list_model_sorted_filtered),
            glib::clone!(
                #[weak_allow_none(rename_to = slf)]
                self,
                move |obj| {
                    if let Some(slf) = slf {
                        let mut cache_brw = slf.imp().row_cache.borrow_mut();
                        let cache = cache_brw.as_mut().unwrap();
                        let wdg: &FieldMonitorCLConnectionEntry = obj.downcast_ref().unwrap();
                        // This cache is needed due to some issues with widget initialization and re-using widgets
                        // when this factory is run again in some states.
                        cache
                            .get_or_insert_ref(wdg, || {
                                gtk::ListBoxRow::builder()
                                    .child(wdg)
                                    .activatable(false)
                                    .selectable(true)
                                    .build()
                                    .upcast()
                            })
                            .clone()
                            .upcast()
                    } else {
                        adw::Bin::new().upcast()
                    }
                }
            ),
        );
    }

    fn on_connection_removed(&self, id: &str) {
        let imp = self.imp();
        let mut list_brw = self.imp().connections.borrow_mut();
        let list = list_brw.as_mut().unwrap();
        if let Entry::Occupied(entry) = list.entry(id.to_string()) {
            let grp = entry.get();
            if let Some(pos) = imp.connection_list_model.find(grp) {
                imp.connection_list_model.remove(pos);
            }
            imp.row_cache
                .borrow_mut()
                .as_mut()
                .map(|cache| cache.pop(grp));
            entry.remove();
        }
        if list.is_empty() {
            self.imp().stack.set_visible_child_name("empty_list");
        }
    }
}

#[gtk::template_callbacks]
impl FieldMonitorConnectionList {
    #[template_callback]
    fn on_search_entry_search_changed(&self) {
        self.imp()
            .connection_list_filter
            .changed(gtk::FilterChange::Different);
    }
}
