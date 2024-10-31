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
use crate::application::FieldMonitorApplication;
use crate::util::ListChange;
use crate::widget::connection_list::info_page::FieldMonitorConnectionInfoPage;
use crate::APP;
use adw::gio;
use adw::prelude::*;
use adw::subclass::prelude::*;
use libfieldmonitor::connection::ConnectionInstance;
use log::debug;
use std::cell::Cell;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorConnectionStack)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/connection_list/connection_stack.ui")]
    pub struct FieldMonitorConnectionStack {
        #[template_child]
        pub outer_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        #[property(get, set)]
        pub toast_overlay: RefCell<Option<adw::ToastOverlay>>,
        #[property(get, set, nullable)]
        pub visible_connection_id: RefCell<Option<String>>,
        pub has_selection: Cell<bool>,
        pub pages: RefCell<Option<gtk::SelectionModel>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorConnectionStack {
        const NAME: &'static str = "FieldMonitorConnectionStack";
        type Type = super::FieldMonitorConnectionStack;
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
    impl ObjectImpl for FieldMonitorConnectionStack {
        fn constructed(&self) {
            self.parent_constructed();
            self.has_selection.set(false);
            let obj = self.obj();
            let mut app_brw = self.application.borrow_mut();

            if app_brw.is_none() {
                *app_brw = Some(APP.with_borrow(|app| app.clone().unwrap()))
            }

            let app = app_brw.clone().unwrap();
            drop(app_brw);

            let connections = app.connections().into_iter().collect::<Vec<_>>();
            for connection in connections {
                obj.on_update_connection(connection);
            }
            app.connect_closure(
                "connection-updated",
                false,
                glib::closure_local!(
                    #[watch]
                    obj,
                    move |_: FieldMonitorApplication, instance: ConnectionInstance| {
                        obj.on_update_connection(instance);
                    }
                ),
            );
            app.connect_closure(
                "connection-removed",
                false,
                glib::closure_local!(
                    #[watch]
                    obj,
                    move |_: FieldMonitorApplication, id: String| {
                        obj.on_connection_removed(&id);
                    }
                ),
            );
            app.connect_notify_local(
                Some("loading-connections"),
                glib::clone!(
                    #[weak(rename_to=slf)]
                    self,
                    move |app, _| {
                        if app.loading_connections() {
                            slf.outer_stack.set_visible_child_name("loading");
                        } else {
                            slf.outer_stack.set_visible_child_name("content");
                        }
                    }
                ),
            );
            app.bind_property("busy", &*obj.imp().stack, "sensitive")
                .invert_boolean()
                .build();
            if !app.loading_connections() {
                self.outer_stack.set_visible_child_name("content");
            }

            // GTK BUG (?):
            // we need to keep a reference, because the stack only keeps a weak reference internally,
            // if we just connect the handler, pages will immediately be dropped again later before
            // anything actually happens
            self.pages.replace(Some(self.stack.pages()));
            self.stack.pages().connect_selection_changed(glib::clone!(
                #[weak]
                obj,
                move |model, position, n_items| {
                    obj.on_pages_model_selection_changed(model, position, n_items);
                }
            ));
        }

        fn dispose(&self) {
            self.pages.replace(None);
        }
    }
    impl WidgetImpl for FieldMonitorConnectionStack {}
    impl BinImpl for FieldMonitorConnectionStack {}
}

glib::wrapper! {
    pub struct FieldMonitorConnectionStack(ObjectSubclass<imp::FieldMonitorConnectionStack>)
        @extends gtk::Widget, adw::Bin,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl FieldMonitorConnectionStack {
    pub fn unselect_connection(&self) {
        if self.imp().has_selection.get() {
            debug!("unselect connection");
            self.imp().has_selection.set(false);
            self.imp().visible_connection_id.replace(None);
            self.notify_visible_connection_id();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.imp().stack.pages().n_items() < 1
    }

    pub fn select_connection(&self, connection: &str) {
        debug!("select connection: {connection}");
        self.imp()
            .visible_connection_id
            .replace(Some(connection.to_string()));
        self.notify_visible_connection_id();
    }

    pub fn pages(&self) -> gtk::SelectionModel {
        self.imp().stack.pages()
    }

    fn on_update_connection(&self, connection: ConnectionInstance) {
        let imp = self.imp();

        let id = connection.connection_id();

        let mut update_type = ListChange::Append;
        for page in imp.stack.pages().iter::<gtk::StackPage>() {
            let item = page.unwrap();
            let item_name = item.name().unwrap_or_default();
            if *item_name == *id {
                update_type = ListChange::Update(item);
                break;
            }
        }

        debug!("add {}: {:?}", connection.title(), update_type);
        match update_type {
            ListChange::Update(page) => {
                page.child()
                    .downcast::<FieldMonitorConnectionInfoPage>()
                    .unwrap()
                    .set_connection(connection.clone());
            }
            _ => {
                let page: gtk::StackPage = imp.stack.add_named(
                    &FieldMonitorConnectionInfoPage::new(&self.application().unwrap(), &connection),
                    Some(&*id),
                );
                connection
                    .bind_property("title", &page, "title")
                    .sync_create()
                    .build();
            }
        }
    }

    fn on_connection_removed(&self, id: &str) {
        let imp = self.imp();
        let currently_selected_name = imp.stack.visible_child_name();

        for page in imp.stack.pages().iter::<gtk::StackPage>() {
            let item = page.unwrap();
            let item_name = item.name().unwrap_or_default();
            if &*item_name == id {
                imp.stack.remove(&item.child());
                break;
            }
        }

        // If was currently selected page: Select no page
        if currently_selected_name.as_deref() == Some(id) {
            self.unselect_connection();
        }
    }
}

#[gtk::template_callbacks]
impl FieldMonitorConnectionStack {
    #[template_callback]
    fn on_self_visible_connection_id_changed(&self) {
        debug!(
            "visible connection id changed: {:?}",
            self.visible_connection_id()
        );
        match self.visible_connection_id() {
            None => {
                self.imp().has_selection.set(false);
            }
            Some(v) => {
                self.imp().has_selection.set(true);
                self.imp().stack.set_visible_child_name(&v);
            }
        }
    }

    fn on_pages_model_selection_changed(
        &self,
        model: &gtk::SelectionModel,
        position: u32,
        n_items: u32,
    ) {
        debug!("selection changed: {position}, {n_items}");
        let selected_page = if !self.imp().has_selection.get() {
            debug!("pretend no selection");
            None
        } else {
            'outer: {
                for i in position..(position + n_items) {
                    if model.is_selected(i) {
                        break 'outer Some(
                            model.item(i).unwrap().downcast::<gtk::StackPage>().unwrap(),
                        );
                    }
                }
                None
            }
        };

        if let Some(name) = selected_page.and_then(|row| row.name()) {
            self.imp()
                .visible_connection_id
                .replace(Some(name.to_string()));
            self.notify_visible_connection_id();
        }
    }
}
