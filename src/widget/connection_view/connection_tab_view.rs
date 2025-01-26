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
use crate::remote_server_info::RemoteServerInfo;
use crate::widget::connection_view::server_screen::FieldMonitorServerScreen;
use crate::widget::window::FieldMonitorWindow;
use crate::APP;
use adw::gio;
use adw::prelude::*;
use adw::subclass::prelude::*;
use itertools::Itertools;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorConnectionTabView)]
    #[template(
        resource = "/de/capypara/FieldMonitor/widget/connection_view/connection_tab_view.ui"
    )]
    pub struct FieldMonitorConnectionTabView {
        #[template_child]
        pub tab_view: TemplateChild<adw::TabView>,
        #[property(get, set)]
        pub toast_overlay: RefCell<Option<adw::ToastOverlay>>,
        #[property(get, set, nullable)]
        pub visible_page: RefCell<Option<adw::TabPage>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorConnectionTabView {
        const NAME: &'static str = "FieldMonitorConnectionTabView";
        type Type = super::FieldMonitorConnectionTabView;
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
    impl ObjectImpl for FieldMonitorConnectionTabView {}
    impl WidgetImpl for FieldMonitorConnectionTabView {}
    impl BinImpl for FieldMonitorConnectionTabView {}
}

glib::wrapper! {
    pub struct FieldMonitorConnectionTabView(ObjectSubclass<imp::FieldMonitorConnectionTabView>)
        @extends gtk::Widget, adw::Bin,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl FieldMonitorConnectionTabView {
    pub fn current(&self) -> Option<FieldMonitorServerScreen> {
        self.visible_page()
            .as_ref()
            .map(adw::TabPage::child)
            .and_downcast::<FieldMonitorServerScreen>()
    }

    /// Try to focus an already open connection view, if a connection view for the given
    /// server is open
    pub fn focus(&self, server_path: &str, adapter_id: &str) -> bool {
        // todo: this could be optimized by storing the page or having a lookup hashmap,
        //       but this is probably fine enough.
        for page in self.imp().tab_view.pages().iter::<adw::TabPage>().flatten() {
            let child = page.child().downcast::<FieldMonitorServerScreen>();
            if let Ok(screen) = child {
                if screen.server_path() == server_path && screen.adapter_id() == adapter_id {
                    self.set_visible_page(Some(page));
                    return true;
                }
            }
        }
        false
    }

    pub fn open(&self, window: &FieldMonitorWindow, info: RemoteServerInfo) {
        let app = window.application().unwrap().downcast().unwrap();
        let view = FieldMonitorServerScreen::new(
            &app,
            Some(window),
            &info.server_path,
            &info.adapter_id,
            info.loader,
        );

        let tab_view = self.imp().tab_view.get();
        let tab = self.add_new_page(&view, &info.server_title, Some(&info.connection_title));
        view.set_close_cb(glib::clone!(
            #[weak]
            tab,
            #[weak]
            tab_view,
            move || {
                tab_view.close_page(&tab);
            }
        ));
    }

    pub fn n_pages(&self) -> u32 {
        self.imp().tab_view.pages().n_items()
    }

    pub fn describe_active(&self) -> Vec<(String, String)> {
        self.imp()
            .tab_view
            .pages()
            .iter::<adw::TabPage>()
            .filter_map_ok(|tab| tab.child().downcast::<FieldMonitorServerScreen>().ok())
            .filter_ok(|view| view.is_connected())
            .map_ok(|view| (view.title(), view.subtitle()))
            .collect::<Result<_, _>>()
            .unwrap_or_default()
    }

    pub(super) fn inner(&self) -> adw::TabView {
        self.imp().tab_view.get()
    }

    fn add_new_page(
        &self,
        page: &impl IsA<gtk::Widget>,
        title: &str,
        subtitle: Option<&str>,
    ) -> adw::TabPage {
        let tab_view = &self.imp().tab_view;

        let page = page.upcast_ref();
        let tab_page = tab_view.append(page);

        if let Some(view) = page.downcast_ref::<FieldMonitorServerScreen>() {
            view.bind_property("title", &tab_page, "title")
                .bidirectional()
                .build();
            if let Some(subtitle) = subtitle {
                view.set_subtitle(subtitle);
            }
        }

        tab_page.set_title(title);

        self.set_visible_page(Some(&tab_page));

        tab_page
    }

    pub fn close_tab(&self, page: &adw::TabPage) {
        self.imp().tab_view.close_page(page);
    }

    pub fn move_page_to_new_window(&self, page: &adw::TabPage) {
        let imp = self.imp();
        let new_window = FieldMonitorWindow::new(&APP.with_borrow(|app| app.clone().unwrap()));

        let child = page.child();
        new_window.set_default_size(child.width(), child.height());

        let tab_view = new_window.tab_view();
        imp.tab_view.transfer_page(page, &tab_view.inner(), 0);

        if let Ok(view) = child.downcast::<FieldMonitorServerScreen>() {
            view.set_close_cb(glib::clone!(
                #[weak]
                page,
                #[weak]
                tab_view,
                move || {
                    tab_view.imp().tab_view.get().close_page(&page);
                }
            ));
        }

        new_window.present();
        new_window.select_connection_view();
    }
}

#[gtk::template_callbacks]
impl FieldMonitorConnectionTabView {
    #[template_callback]
    fn on_self_visible_page_changed(&self) {
        let brw = self.imp().visible_page.borrow();
        if let Some(page) = brw.as_ref() {
            let page = page.clone();
            drop(brw);
            self.imp().tab_view.set_selected_page(&page);
        }
    }

    #[template_callback]
    fn on_tab_view_create_window(&self, _tab_view: &adw::TabView) -> adw::TabView {
        let new_window = FieldMonitorWindow::new(&APP.with_borrow(|app| app.clone().unwrap()));
        new_window.present();
        new_window.select_connection_view();
        new_window.tab_view().inner().clone()
    }

    #[template_callback]
    fn on_tab_view_selected_page_changed(&self) {
        let new_page = self.imp().tab_view.selected_page();
        if let Some(new_page) = new_page {
            if self.visible_page().as_ref() == Some(&new_page) {
                self.set_visible_page(Some(&new_page))
            }
        }
    }

    #[template_callback]
    fn on_tab_view_close_page(&self, page: &adw::TabPage) -> glib::Propagation {
        if self.visible_page().as_ref() == Some(page) {
            self.set_visible_page(None::<&adw::TabPage>);
        }
        // TODO: confirmation dialog
        glib::Propagation::Proceed
    }

    #[template_callback]
    fn on_tab_view_page_detached(&self, page: &adw::TabPage) {
        if self.visible_page().as_ref() == Some(page)
            || self.imp().tab_view.selected_page().is_none()
        {
            self.set_visible_page(None::<&adw::TabPage>);
        }
    }
}
