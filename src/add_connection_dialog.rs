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

use std::rc::Rc;

use adw::prelude::{ActionRowExt, NavigationPageExt};
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::{ButtonExt, WidgetExt};

use crate::application::FieldMonitorApplication;
use crate::connection::CONNECTION_PROVIDERS;
use crate::connection::types::ConnectionProvider;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/de/capypara/FieldMonitor/add_connection_dialog.ui")]
    pub struct FieldMonitorAddConnectionDialog {
        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub actions: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorAddConnectionDialog {
        const NAME: &'static str = "FieldMonitorAddConnectionDialog";
        type Type = super::FieldMonitorAddConnectionDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FieldMonitorAddConnectionDialog {}
    impl WidgetImpl for FieldMonitorAddConnectionDialog {}
    impl AdwDialogImpl for FieldMonitorAddConnectionDialog {}
}

glib::wrapper! {
    pub struct FieldMonitorAddConnectionDialog(ObjectSubclass<imp::FieldMonitorAddConnectionDialog>)
        @extends gtk::Widget, adw::Dialog;
}

impl FieldMonitorAddConnectionDialog {
    pub fn new(app: &FieldMonitorApplication) -> Self {
        let slf: Self = glib::Object::builder().build();

        for constructor in CONNECTION_PROVIDERS {
            let provider = Rc::new(constructor.new(app));
            let action_row = adw::ActionRow::builder()
                .title(provider.title())
                .subtitle(provider.description())
                .activatable(true)
                .build();
            action_row.connect_activated(clone!(@weak slf =>
                move |_| {
                    slf.on_activate_provider(provider.clone());
                }
            ));
            let next_image = gtk::Image::builder().icon_name("go-next-symbolic").build();
            next_image.add_css_class("dim-label");
            action_row.add_suffix(&next_image);
            slf.imp().actions.append(&action_row);
        }

        slf
    }
}

#[gtk::template_callbacks]
impl FieldMonitorAddConnectionDialog {}

impl FieldMonitorAddConnectionDialog {
    fn on_activate_provider(&self, provider: Rc<Box<dyn ConnectionProvider>>) {
        let add_button = gtk::Button::builder().label(gettext("Add")).build();
        add_button.add_css_class("suggested-action");

        let action_bar = gtk::ActionBar::new();
        action_bar.pack_end(&add_button);

        let preferences = provider.preferences(None);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.add_bottom_bar(&action_bar);
        toolbar.set_content(Some(&preferences));
        let settings_nav_page = adw::NavigationPage::builder()
            .title(provider.add_title())
            .child(&toolbar)
            .build();

        let slf = self;
        add_button.connect_clicked(clone!(@weak slf, @weak preferences =>
            move |_| {
                slf.on_connection_add((*provider).as_ref(), preferences);
            }
        ));

        self.imp().navigation_view.push(&settings_nav_page);
    }

    fn on_connection_add(
        &self,
        provider: &dyn ConnectionProvider,
        configured_preferences: gtk::Widget,
    ) {
        todo!()
    }
}
