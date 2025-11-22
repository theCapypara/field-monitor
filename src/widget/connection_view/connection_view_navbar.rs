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
use crate::widget::connection_view::FieldMonitorConnectionTabView;
use adw::gio;
use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::widget::navbar_row::FieldMonitorNavbarRow;
use log::{debug, warn};
use std::cell::RefCell;
use std::collections::HashMap;

mod imp {
    use super::*;
    use adw::gdk::pango;
    use gettextrs::gettext;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorNavbarConnectionView)]
    #[template(
        resource = "/de/capypara/FieldMonitor/widget/connection_view/connection_view_navbar.ui"
    )]
    pub struct FieldMonitorNavbarConnectionView {
        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub header_label: TemplateChild<gtk::Label>,
        #[property(get, set = Self::set_tab_view)]
        pub tab_view: RefCell<Option<FieldMonitorConnectionTabView>>,
        pub inner_tab_view: RefCell<Option<adw::TabView>>,
        pub(super) rows: RefCell<HashMap<adw::TabPage, FieldMonitorNavbarRow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorNavbarConnectionView {
        const NAME: &'static str = "FieldMonitorNavbarConnectionView";
        type Type = super::FieldMonitorNavbarConnectionView;
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
    impl ObjectImpl for FieldMonitorNavbarConnectionView {}
    impl WidgetImpl for FieldMonitorNavbarConnectionView {}
    impl BinImpl for FieldMonitorNavbarConnectionView {}

    impl FieldMonitorNavbarConnectionView {
        fn set_tab_view(&self, tab_view: Option<FieldMonitorConnectionTabView>) {
            match tab_view {
                None => self.unset_tab_view(),
                Some(tab_view) => {
                    if self.tab_view.borrow().as_ref() == Some(&tab_view) {
                        return;
                    }
                    self.unset_tab_view();
                    self.do_set_tab_view(tab_view);
                    self.obj().queue_resize();
                }
            }
        }

        fn unset_tab_view(&self) {
            // tab_view.take removes the old tab_view.
            if self.tab_view.take().is_some() {
                self.clear_sidebar();
                // TODO: Remove signal handlers?
            }
        }

        fn do_set_tab_view(&self, tab_view: FieldMonitorConnectionTabView) {
            self.tab_view.replace(Some(tab_view.clone()));
            let inner_tab_view = tab_view.inner();
            self.inner_tab_view.replace(Some(inner_tab_view.clone()));
            self.populate_sidebar();
            inner_tab_view.connect_page_attached(glib::clone!(
                #[weak(rename_to=slf)]
                self,
                move |_, _, _| {
                    slf.clear_sidebar();
                    slf.populate_sidebar();
                }
            ));
            inner_tab_view.connect_page_detached(glib::clone!(
                #[weak(rename_to=slf)]
                self,
                move |_, _, _| {
                    slf.clear_sidebar();
                    slf.populate_sidebar();
                }
            ));
            inner_tab_view.connect_page_reordered(glib::clone!(
                #[weak(rename_to=slf)]
                self,
                move |_, _, _| {
                    slf.clear_sidebar();
                    slf.populate_sidebar();
                }
            ));
            tab_view.connect_visible_page_notify(glib::clone!(
                #[weak(rename_to=slf)]
                self,
                move |tab_view| {
                    slf.obj()
                        .on_tab_view_visible_page_changed(tab_view.visible_page());
                }
            ));
            self.obj()
                .on_tab_view_visible_page_changed(tab_view.visible_page());
        }

        pub fn clear_sidebar(&self) {
            // `take` will basically clear the hash map.
            for (_, row) in self.rows.take() {
                self.list.remove(&row);
            }
        }

        pub fn populate_sidebar(&self) {
            let tab_view_brw = self.tab_view.borrow();
            let inner_brw = self.inner_tab_view.borrow();
            let mut self_empty = true;
            if let (Some(inner), Some(tab_view)) = (inner_brw.as_ref(), tab_view_brw.as_ref()) {
                let active_page = tab_view.visible_page();

                let mut rows_brw = self.rows.borrow_mut();
                for page in inner.pages().iter() {
                    self_empty = false;
                    let page: adw::TabPage = page.unwrap();

                    let item = gtk::Label::builder()
                        .label("")
                        .halign(gtk::Align::Start)
                        .hexpand(true)
                        .valign(gtk::Align::Center)
                        .max_width_chars(20)
                        .wrap(false)
                        .ellipsize(pango::EllipsizeMode::End)
                        .build();
                    let row: FieldMonitorNavbarRow = glib::Object::builder()
                        .property("child-ref", &page)
                        .property("content", &item)
                        .property("selectable", false)
                        .property("activatable", true)
                        .build();
                    row.update_relation(&[gtk::accessible::Relation::LabelledBy(&[
                        item.upcast_ref()
                    ])]);

                    let close_button = gtk::Button::builder()
                        .icon_name("cross-small-symbolic")
                        .tooltip_text(gettext("Close"))
                        .valign(gtk::Align::Center)
                        .css_classes(["flat", "compact-button"])
                        .build();
                    close_button.set_action_name(Some("row.view-close"));
                    row.add_suffix(&close_button);

                    row.add_context_menu(|| {
                        let menu = gio::Menu::new();

                        menu.append_item(&gio::MenuItem::new(
                            Some(&gettext("Move to New Window")),
                            Some("row.view-move-to-new-window"),
                        ));
                        let close_item =
                            gio::MenuItem::new(Some(&gettext("Close")), Some("row.view-close"));
                        menu.append_item(&close_item);
                        menu
                    });

                    row.add_row_action(
                        "view-move-to-new-window",
                        glib::clone!(
                            #[weak]
                            tab_view,
                            #[weak]
                            page,
                            move |_| tab_view.move_page_to_new_window(&page)
                        ),
                    );
                    row.add_row_action(
                        "view-close",
                        glib::clone!(
                            #[weak]
                            tab_view,
                            #[weak]
                            page,
                            move |_| tab_view.close_tab(&page)
                        ),
                    );

                    page.bind_property("title", &item, "label")
                        .sync_create()
                        .build();

                    let is_selected = Some(page) == active_page;

                    if is_selected {
                        self.list.select_row(Some(&row));
                    } else {
                        self.list.unselect_row(&row);
                    }

                    self.list.append(&row);
                    rows_brw.insert(
                        row.child_ref().unwrap().downcast::<adw::TabPage>().unwrap(),
                        row,
                    );
                }
            }

            self.obj().set_visible(!self_empty);
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorNavbarConnectionView(ObjectSubclass<imp::FieldMonitorNavbarConnectionView>)
        @extends gtk::Widget, adw::Bin,
        @implements gio::ActionGroup, gio::ActionMap, gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

#[gtk::template_callbacks]
impl FieldMonitorNavbarConnectionView {
    #[template_callback]
    fn on_list_row_activated(&self, row: &FieldMonitorNavbarRow) {
        let page = row.child_ref().unwrap().downcast::<adw::TabPage>().unwrap();
        debug!("list row activated: {:?}", page);
        if let Some(tab_view) = self.imp().tab_view.borrow().as_ref() {
            tab_view.set_visible_page(Some(page));
        } else {
            warn!("no tab_view?");
        }
    }

    fn on_tab_view_visible_page_changed(&self, page: Option<adw::TabPage>) {
        debug!("tab view nav bar: on_tab_view_visible_page_changed");
        for row in self.imp().rows.borrow().values() {
            row.remove_css_class("fm-navselected");
        }
        if let Some(page) = page {
            let rows_brw = self.imp().rows.borrow();
            let row = rows_brw.get(&page);
            if let Some(row) = row {
                row.add_css_class("fm-navselected");
            } else {
                warn!("unknown page selected: {page:?}");
            }
        }
    }
}
