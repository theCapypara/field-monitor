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
use crate::util::OrdKeyed;
use crate::widget::connection_list::DEFAULT_GENERIC_ICON;
use crate::widget::connection_list::FieldMonitorConnectionStack;
use crate::widget::connection_list::info_page::FieldMonitorConnectionInfoPage;
use crate::widget::navbar_row::FieldMonitorNavbarRow;
use adw::gio;
use adw::prelude::*;
use adw::subclass::prelude::*;
use libfieldmonitor::connection::*;
use log::{debug, warn};
use sorted_vec::SortedSet;
use std::cell::RefCell;
use std::collections::HashMap;

mod imp {
    use super::*;
    use futures::future::OptionFuture;
    use futures::{StreamExt, stream};
    use gettextrs::gettext;
    use gtk::pango;
    use std::sync::Arc;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorNavbarConnectionList)]
    #[template(
        resource = "/de/capypara/FieldMonitor/widget/connection_list/connection_list_navbar.ui"
    )]
    pub struct FieldMonitorNavbarConnectionList {
        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,
        #[property(get, nullable, set = Self::set_stack)]
        pub stack: RefCell<Option<FieldMonitorConnectionStack>>,
        pub pages: RefCell<Option<gtk::SelectionModel>>,
        pub(super) rows: RefCell<HashMap<String, RowEntry>>, // key is connection ID
        pub stack_active_changed_handler_id: RefCell<Option<glib::SignalHandlerId>>,
        pub pages_items_changed_handler_id: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorNavbarConnectionList {
        const NAME: &'static str = "FieldMonitorNavbarConnectionList";
        type Type = super::FieldMonitorNavbarConnectionList;
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
    impl ObjectImpl for FieldMonitorNavbarConnectionList {
        fn dispose(&self) {
            self.unset_stack();
        }
    }
    impl WidgetImpl for FieldMonitorNavbarConnectionList {}
    impl BinImpl for FieldMonitorNavbarConnectionList {}

    impl FieldMonitorNavbarConnectionList {
        fn set_stack(&self, stack: Option<FieldMonitorConnectionStack>) {
            match stack {
                None => self.unset_stack(),
                Some(stack) => {
                    if self.stack.borrow().as_ref() == Some(&stack) {
                        return;
                    }
                    self.unset_stack();
                    let obj = self.obj();
                    glib::spawn_future_local(glib::clone!(
                        #[strong]
                        obj,
                        async move {
                            obj.imp().do_set_stack(stack).await;
                            obj.queue_resize();
                        }
                    ));
                }
            }
        }

        fn unset_stack(&self) {
            // stack.take removes the old stack.
            if let Some(stack) = self.stack.take() {
                self.clear_sidebar();
                let page = self.pages.take();
                let handler = self.pages_items_changed_handler_id.take();
                if let (Some(page), Some(handler)) = (page, handler) {
                    page.disconnect(handler);
                }
                let handler = self.stack_active_changed_handler_id.take();
                if let Some(handler) = handler {
                    stack.disconnect(handler);
                }
            }
        }

        async fn do_set_stack(&self, stack: FieldMonitorConnectionStack) {
            let pages = stack.pages();
            self.stack.replace(Some(stack.clone()));
            self.pages.replace(Some(pages.clone()));
            self.populate_sidebar().await;
            self.pages_items_changed_handler_id
                .replace(Some(pages.connect_items_changed(glib::clone!(
                    #[weak(rename_to=slf)]
                    self,
                    move |model, position, removed, added| {
                        slf.obj()
                            .on_pages_items_changed(model, position, removed, added)
                    }
                ))));
            self.stack_active_changed_handler_id.replace(Some(
                stack.connect_visible_connection_id_notify(glib::clone!(
                    #[weak(rename_to=slf)]
                    self,
                    move |stack| {
                        slf.obj().on_stack_visible_connection_id_changed(
                            stack.visible_connection_id().as_deref(),
                        );
                    }
                )),
            ));
            self.obj()
                .on_stack_visible_connection_id_changed(stack.visible_connection_id().as_deref());
        }

        pub fn clear_sidebar(&self) {
            // `take` will basically clear the hash map.
            for (_, row) in self.rows.take() {
                row.page.disconnect(row.handler);
                self.list.remove(&row.row);
            }
        }

        pub async fn populate_sidebar(&self) {
            let pages_opt = self.pages.borrow().clone();
            let stack_opt = self.stack.borrow().clone();
            if let (Some(pages), Some(stack)) = (pages_opt, stack_opt) {
                let active_connection = Arc::new(stack.visible_connection_id());

                let sorted_rows = stream::iter(pages.iter())
                    .then(|page| {
                        let active_connection = active_connection.clone();
                        async move {
                            let page: gtk::StackPage = page.unwrap();
                            let item = gtk::Label::builder()
                                .label("")
                                .halign(gtk::Align::Start)
                                .valign(gtk::Align::Center)
                                .max_width_chars(20)
                                .wrap(false)
                                .ellipsize(pango::EllipsizeMode::End)
                                .build();
                            let row: FieldMonitorNavbarRow = glib::Object::builder()
                                .property(
                                    "child-ref",
                                    gtk::StringObject::new(&page.name().unwrap_or_default()),
                                )
                                .property("content", &item)
                                .property("selectable", false)
                                .property("activatable", true)
                                .build();
                            row.update_relation(&[gtk::accessible::Relation::LabelledBy(&[
                                item.upcast_ref()
                            ])]);

                            row.add_context_menu(|| {
                                let menu = gio::Menu::new();
                                let main_section = gio::Menu::new();
                                let close_section = gio::Menu::new();

                                main_section.append_item(&gio::MenuItem::new(
                                    Some(&gettext("Edit Connection")),
                                    Some("row.connection-edit"),
                                ));

                                let close_item = gio::MenuItem::new(
                                    Some(&gettext("Remove Connection")),
                                    Some("row.connection-remove"),
                                );
                                close_section.append_item(&close_item);

                                menu.append_section(None, &main_section);
                                menu.append_section(None, &close_section);

                                menu
                            });

                            row.add_row_action(
                                "connection-edit",
                                glib::clone!(
                                    #[weak]
                                    page,
                                    move |row| {
                                        row.activate_action(
                                            "app.edit-connection",
                                            Some(&page.name().unwrap_or_default().to_variant()),
                                        )
                                        .ok();
                                    }
                                ),
                            );
                            row.add_row_action(
                                "connection-remove",
                                glib::clone!(
                                    #[weak]
                                    page,
                                    move |row| {
                                        row.activate_action(
                                            "app.remove-connection",
                                            Some(&page.name().unwrap_or_default().to_variant()),
                                        )
                                        .ok();
                                    }
                                ),
                            );

                            let conn_meta = OptionFuture::from(
                                page.child()
                                    .downcast_ref::<FieldMonitorConnectionInfoPage>()
                                    .and_then(FieldMonitorConnectionInfoPage::connection)
                                    .as_ref()
                                    .map(ConnectionInstance::metadata),
                            )
                            .await;
                            let icon_spec = conn_meta
                                .as_ref()
                                .map(|m| m.icon.clone())
                                .unwrap_or_else(|| IconSpec::Named("dialog-error-symbolic".into()));

                            let icon: gtk::Widget = match icon_spec {
                                IconSpec::Default => gtk::Image::builder()
                                    .icon_name(DEFAULT_GENERIC_ICON)
                                    .build()
                                    .upcast(),
                                IconSpec::None => {
                                    gtk::Box::builder().width_request(16).build().upcast()
                                }
                                IconSpec::Named(name) => {
                                    gtk::Image::builder().icon_name(&*name).build().upcast()
                                }
                                IconSpec::Custom(factory) => factory(&conn_meta.unwrap()),
                            };

                            row.add_prefix(&icon);

                            self.update_row(&page, &row);

                            let is_selected =
                                page.name().as_deref() == active_connection.as_deref();

                            OrdKeyed(
                                page.title().map(|v| v.to_lowercase()),
                                (page, row, is_selected),
                            )
                        }
                    })
                    .collect::<SortedSet<_>>()
                    .await;

                let mut rows_brw = self.rows.borrow_mut();
                for (i, key) in sorted_rows.into_vec().into_iter().enumerate() {
                    let (page, row, is_selected) = key.1;

                    if is_selected {
                        debug!("select row {i}");
                        self.list.select_row(Some(&row));
                    } else {
                        self.list.unselect_row(&row);
                    }

                    self.list.append(&row);
                    let handler = page.connect_notify_local(
                        None,
                        glib::clone!(
                            #[weak(rename_to=slf)]
                            self,
                            move |page, _| {
                                slf.obj().on_page_updated(page);
                            }
                        ),
                    );
                    rows_brw.insert(row_string(&row), RowEntry { row, page, handler });
                }
            }
        }

        pub fn update_row(&self, page: &gtk::StackPage, row: &FieldMonitorNavbarRow) {
            let item = row.content().unwrap().downcast::<gtk::Label>().unwrap();
            let title = page.title();
            item.set_visible(row.is_visible() && title.is_some());
            if let Some(title) = title {
                item.set_label(&title);
            }
            if page.needs_attention() {
                row.add_css_class("needs-attention");
            } else {
                row.remove_css_class("needs-attention");
            }
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorNavbarConnectionList(ObjectSubclass<imp::FieldMonitorNavbarConnectionList>)
        @extends gtk::Widget, adw::Bin,
        @implements gio::ActionGroup, gio::ActionMap, gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

#[gtk::template_callbacks]
impl FieldMonitorNavbarConnectionList {
    #[template_callback]
    fn on_list_row_activated(&self, row: &FieldMonitorNavbarRow) {
        let child_name = row_string(row);
        debug!("list row activated: {:?}", child_name);
        if let Some(stack) = self.imp().stack.borrow().as_ref() {
            stack.set_visible_connection_id(Some(child_name));
        } else {
            warn!("no stack?");
        }
    }

    fn on_pages_items_changed(
        &self,
        _model: &gtk::SelectionModel,
        _position: u32,
        _removed: u32,
        _added: u32,
    ) {
        self.imp().clear_sidebar();
        glib::spawn_future_local(glib::clone!(
            #[strong(rename_to=slf)]
            self,
            async move {
                slf.imp().populate_sidebar().await;
            }
        ));
    }

    fn on_page_updated(&self, _page: &gtk::StackPage) {
        // the title could change :(
        self.imp().clear_sidebar();
        glib::spawn_future_local(glib::clone!(
            #[strong(rename_to=slf)]
            self,
            async move {
                slf.imp().populate_sidebar().await;
            }
        ));
    }

    fn on_stack_visible_connection_id_changed(&self, connection_id: Option<&str>) {
        for row in self.imp().rows.borrow().values() {
            row.row.remove_css_class("fm-navselected");
        }
        if let Some(connection_id) = connection_id {
            let rows_brw = self.imp().rows.borrow();
            let row = rows_brw.get(connection_id);
            if let Some(row) = row {
                row.row.add_css_class("fm-navselected");
            } else {
                warn!("unknown page selected: {connection_id}");
            }
        }
    }
}

#[derive(Debug)]
struct RowEntry {
    row: FieldMonitorNavbarRow,
    handler: glib::SignalHandlerId,
    page: gtk::StackPage,
}

fn row_string(row: &FieldMonitorNavbarRow) -> String {
    row.child_ref()
        .unwrap()
        .downcast::<gtk::StringObject>()
        .unwrap()
        .string()
        .into()
}
