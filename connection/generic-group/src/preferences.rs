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

use std::cell::Cell;
use std::cell::RefCell;
use std::num::NonZeroU32;
use std::ops::{Deref, DerefMut};

use adw::gio;
use adw::prelude::*;
use adw::subclass::prelude::*;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use log::warn;
use secure_string::SecureString;
use uuid::Uuid;

use libfieldmonitor::connection::*;
use libfieldmonitor::i18n::gettext_f;

use crate::server_config::{ServerConfigChanges, ServerConfigForRow};
use crate::server_preferences::GenericGroupServerPreferences;

pub trait GenericGroupConfiguration {
    fn connection_title(&self) -> Option<&str>;
    fn title(&self, server: &str) -> Option<String>;
    fn host(&self, server: &str) -> Option<String>;
    fn port(&self, server: &str) -> Option<NonZeroU32>;
    fn user(&self, server: &str) -> Option<String>;
    fn password(&self, server: &str) -> LocalBoxFuture<anyhow::Result<Option<SecureString>>>;
    fn set_connection_title(&mut self, value: &str);
    fn set_title(&mut self, server: &str, value: &str);
    fn set_host(&mut self, server: &str, value: &str);
    fn set_port(&mut self, server: &str, value: NonZeroU32);
    fn set_user(&mut self, server: &str, value: Option<&str>);
    fn set_password(&mut self, server: &str, value: Option<SecureString>);
    fn set_password_session(&mut self, server: &str, value: Option<&SecureString>);
    fn remove_server(&mut self, server: &str);
}

impl GenericGroupConfiguration for ConnectionConfiguration {
    fn connection_title(&self) -> Option<&str> {
        self.get_try_as_str("title")
    }

    fn title(&self, server: &str) -> Option<String> {
        self.with_section(server, |section| section.get_try_as_string("title"))
    }

    fn host(&self, server: &str) -> Option<String> {
        self.with_section(server, |section| section.get_try_as_string("host"))
    }

    fn port(&self, server: &str) -> Option<NonZeroU32> {
        self.with_section(server, |section| {
            section.get_try_as_u64("port").and_then(|v| {
                if v <= (u32::MAX as u64) {
                    NonZeroU32::new(v as u32)
                } else {
                    None
                }
            })
        })
    }

    fn user(&self, server: &str) -> Option<String> {
        self.with_section(server, |section| section.get_try_as_string("user"))
    }

    fn password(&self, server: &str) -> LocalBoxFuture<anyhow::Result<Option<SecureString>>> {
        let server = server.to_string();
        Box::pin(async move {
            self.with_section_async(&server, |section| {
                Box::pin(async move {
                    if let Some(pw) = section.get_try_as_sec_string("__session__password") {
                        return Ok(Some(pw));
                    }
                    section.get_secret("password").await
                })
            })
            .await
        })
    }

    fn set_connection_title(&mut self, value: &str) {
        self.set_value("title", value);
    }

    fn set_title(&mut self, server: &str, value: &str) {
        self.with_section_mut(server, |mut section| section.set_value("title", value));
    }

    fn set_host(&mut self, server: &str, value: &str) {
        self.with_section_mut(server, |mut section| section.set_value("host", value));
    }

    fn set_port(&mut self, server: &str, value: NonZeroU32) {
        self.with_section_mut(server, |mut section| section.set_value("port", value.get()));
    }

    fn set_user(&mut self, server: &str, value: Option<&str>) {
        let value = match value {
            None => serde_yaml::Value::Null,
            Some(value) => value.into(),
        };
        self.with_section_mut(server, |mut section| section.set_value("user", value));
    }

    fn set_password(&mut self, server: &str, value: Option<SecureString>) {
        self.set_password_session(server, value.as_ref());
        self.with_section_mut(server, |mut section| match value {
            None => section.clear_secret("password"),
            Some(value) => section.set_secret("password", value),
        })
    }

    fn set_password_session(&mut self, server: &str, value: Option<&SecureString>) {
        self.with_section_mut(server, |mut section| match value {
            None => {
                section.clear("__session__password");
            }
            Some(value) => {
                section.set_secure_string("__session__password", value.clone());
            }
        })
    }

    fn remove_server(&mut self, server: &str) {
        if let Some(ConfigValueRef::SerdeValue(serde_yaml::Value::Mapping(_))) = self.get(server) {
            self.clear(server)
        }
    }
}

impl<T: GenericGroupConfiguration + ?Sized> GenericGroupConfiguration for Box<T> {
    fn connection_title(&self) -> Option<&str> {
        self.deref().connection_title()
    }

    fn title(&self, server: &str) -> Option<String> {
        self.deref().title(server)
    }

    fn host(&self, server: &str) -> Option<String> {
        self.deref().host(server)
    }

    fn port(&self, server: &str) -> Option<NonZeroU32> {
        self.deref().port(server)
    }

    fn user(&self, server: &str) -> Option<String> {
        self.deref().user(server)
    }

    fn password(&self, server: &str) -> LocalBoxFuture<anyhow::Result<Option<SecureString>>> {
        self.deref().password(server)
    }

    fn set_connection_title(&mut self, value: &str) {
        self.deref_mut().set_connection_title(value)
    }

    fn set_title(&mut self, server: &str, value: &str) {
        self.deref_mut().set_title(server, value)
    }

    fn set_host(&mut self, server: &str, value: &str) {
        self.deref_mut().set_host(server, value)
    }

    fn set_port(&mut self, server: &str, value: NonZeroU32) {
        self.deref_mut().set_port(server, value)
    }

    fn set_user(&mut self, server: &str, value: Option<&str>) {
        self.deref_mut().set_user(server, value)
    }

    fn set_password(&mut self, server: &str, value: Option<SecureString>) {
        self.deref_mut().set_password(server, value)
    }

    fn set_password_session(&mut self, server: &str, value: Option<&SecureString>) {
        self.deref_mut().set_password_session(server, value)
    }

    fn remove_server(&mut self, server: &str) {
        self.deref_mut().remove_server(server)
    }
}

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::GenericGroupPreferences)]
    #[template(resource = "/de/capypara/FieldMonitor/connection/generic-group/preferences.ui")]
    pub struct GenericGroupPreferences {
        #[template_child]
        pub server_store: TemplateChild<gio::ListStore>,
        #[template_child]
        pub server_store_sorted: TemplateChild<gtk::SortListModel>,
        #[template_child]
        pub title_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub servers_box: TemplateChild<gtk::ListBox>,
        #[property(get, set)]
        pub title: RefCell<String>,
        pub server_model_bound: Cell<bool>,
        pub changes: RefCell<ServerConfigChanges>,
        pub config: RefCell<Option<ConnectionConfiguration>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GenericGroupPreferences {
        const NAME: &'static str = "GenericGroupPreferences";
        type Type = super::GenericGroupPreferences;
        type ParentType = adw::PreferencesPage;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for GenericGroupPreferences {}
    impl WidgetImpl for GenericGroupPreferences {}
    impl PreferencesPageImpl for GenericGroupPreferences {}
}

glib::wrapper! {
    pub struct GenericGroupPreferences(ObjectSubclass<imp::GenericGroupPreferences>)
        @extends gtk::Widget, adw::PreferencesPage;
}

impl GenericGroupPreferences {
    pub fn new(existing_configuration: Option<&ConnectionConfiguration>) -> Self {
        let slf: Self = glib::Object::builder().build();

        if let Some(existing_configuration) = existing_configuration.cloned() {
            slf.set_title(
                existing_configuration
                    .connection_title()
                    .unwrap_or_default(),
            );

            let servers: Vec<ServerConfigForRow> = existing_configuration
                .section_keys()
                .map(|section_key| {
                    let title = existing_configuration
                        .title(section_key)
                        .unwrap_or_default();
                    let host = existing_configuration.host(section_key).unwrap_or_default();

                    glib::Object::builder()
                        .property("key", section_key)
                        .property("title", title)
                        .property("host", host)
                        .property("port", existing_configuration.port(section_key))
                        .property("user", existing_configuration.user(section_key))
                        .build()
                })
                .collect();

            if servers.is_empty() {
                slf.imp().servers_box.remove_all();
                slf.add_dummy_row();
            } else {
                slf.imp().server_store.extend_from_slice(&servers);
                slf.bind_model();
            }

            slf.imp().config.replace(Some(existing_configuration));
        } else {
            slf.add_dummy_row();
        }
        slf
    }

    fn bind_model(&self) {
        let imp = self.imp();
        if imp.server_model_bound.get() {
            return;
        }
        imp.server_model_bound.set(true);
        imp.servers_box.remove_all();

        let property_expr = gtk::PropertyExpression::new(
            ServerConfigForRow::static_type(),
            None::<&gtk::Expression>,
            "title",
        );
        imp.server_store_sorted
            .set_sorter(Some(&gtk::StringSorter::new(Some(&property_expr))));

        imp.servers_box.bind_model(
            Some(&*imp.server_store_sorted),
            glib::clone!(
                #[strong(rename_to = slf)]
                self,
                move |obj| {
                    let config: &ServerConfigForRow = obj.downcast_ref().unwrap();
                    let edit = gtk::Button::builder()
                        .icon_name("edit-symbolic")
                        .css_classes(["flat"])
                        .build();
                    let cb_key = config.key();
                    edit.connect_clicked(glib::clone!(
                        #[weak]
                        slf,
                        move |_| slf.add_or_edit_server(false, &cb_key)
                    ));
                    let user_part = config
                        .user()
                        .filter(|u| !u.is_empty())
                        .map(|u| format!("{u}@"))
                        .unwrap_or_default();
                    let row = adw::ActionRow::builder()
                        .title(config.title())
                        .subtitle(format!("{}{}:{}", user_part, config.host(), config.port()))
                        .activatable_widget(&edit)
                        .build();
                    row.add_suffix(&edit);
                    row.upcast()
                }
            ),
        );
    }

    fn add_dummy_row(&self) {
        let imp = self.imp();
        imp.servers_box.append(
            &adw::ActionRow::builder()
                .sensitive(false)
                .title(gettext("No servers added yet."))
                .build(),
        )
    }

    pub(crate) fn servers(&self) -> impl Deref<Target = ServerConfigChanges> + '_ {
        self.imp().changes.borrow()
    }

    pub fn add_server(&self) {
        let id = Uuid::now_v7().to_string();
        self.add_or_edit_server(true, &id);
    }

    pub fn add_or_edit_server(&self, is_new: bool, key: &str) {
        let borrowed_cfg = self.imp().config.borrow();
        let stored_cfg = borrowed_cfg.as_ref();
        let cfg_base = self.imp().changes.borrow().updates.clone();

        let cfg: Box<dyn GenericGroupConfiguration> = match stored_cfg {
            None => Box::new(cfg_base),
            Some(stored_cfg) => Box::new(cfg_base.either_or(stored_cfg.clone())),
        };
        self.present_server_window(
            is_new,
            key,
            cfg,
            glib::clone!(
                #[strong(rename_to = slf)]
                self,
                move |editor| {
                    let cfg = editor.make_config()?;
                    // Insert or update store
                    let mut found = false;
                    for server in slf.imp().server_store.iter::<glib::Object>() {
                        let server = server.unwrap().downcast::<ServerConfigForRow>().unwrap();
                        if server.key() == cfg.key {
                            found = true;
                            server.set_title(&*cfg.title);
                            server.set_host(&*cfg.host);
                            server.set_port(u32::from(cfg.port));
                            server.set_user(cfg.user.as_deref());
                            break;
                        }
                    }
                    if !found {
                        slf.imp().server_store.append(
                            &glib::Object::builder::<ServerConfigForRow>()
                                .property("key", &cfg.key)
                                .property("title", &cfg.title)
                                .property("host", &cfg.host)
                                .property("port", cfg.port)
                                .property("user", cfg.user.as_deref())
                                .build(),
                        )
                    }
                    // Insert into changes
                    slf.imp()
                        .changes
                        .borrow_mut()
                        .updates
                        .insert(cfg.key.clone(), cfg);

                    slf.bind_model();
                    Some(())
                }
            ),
            glib::clone!(
                #[strong(rename_to = slf)]
                self,
                move |key| {
                    // Remove from store
                    let mut to_remove = None;
                    for (pos, server) in slf.imp().server_store.iter::<glib::Object>().enumerate() {
                        let server = server.unwrap().downcast::<ServerConfigForRow>().unwrap();
                        if server.key() == key {
                            to_remove = Some(pos);
                            break;
                        }
                    }
                    if let Some(to_remove) = to_remove {
                        slf.imp().server_store.remove(to_remove as u32);
                    }
                    // Insert into changes
                    slf.imp().changes.borrow_mut().removes.push(key.to_string());
                    Some(())
                }
            ),
        );
    }

    fn present_server_window<T>(
        &self,
        is_new: bool,
        id: &str,
        config: T,
        on_save: impl Fn(&GenericGroupServerPreferences) -> Option<()> + 'static,
        on_remove: impl Fn(&str) -> Option<()> + 'static,
    ) where
        T: GenericGroupConfiguration + 'static,
    {
        let title = if !is_new {
            gettext_f(
                "Edit {title}",
                &[("title", &config.title(id).unwrap_or_default())],
            )
        } else {
            gettext("Add Server")
        };
        let editor =
            GenericGroupServerPreferences::new(id, if !is_new { Some(config) } else { None });
        let dialog = adw::Dialog::builder().title(title).build();
        let bottom_bar = gtk::ActionBar::new();
        let save_button = gtk::Button::builder()
            .label(if !is_new {
                gettext("Update")
            } else {
                gettext("Add")
            })
            .css_classes(["suggested-action"])
            .build();
        save_button.connect_clicked(glib::clone!(
            #[strong]
            editor,
            #[strong]
            dialog,
            move |_| {
                match on_save(&editor) {
                    Some(()) => dialog.force_close(),
                    None => warn!("Error saving server."),
                };
            }
        ));
        bottom_bar.pack_end(&save_button);
        if !is_new {
            let remove_button = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .tooltip_text(gettext("Remove Server"))
                .css_classes(["destructive-action"])
                .build();
            let id_cln = id.to_string();
            remove_button.connect_clicked(glib::clone!(
                #[strong]
                dialog,
                move |_| {
                    on_remove(&id_cln);
                    dialog.force_close();
                }
            ));
            bottom_bar.pack_start(&remove_button);
        }

        let view = adw::ToolbarView::new();
        view.add_top_bar(&adw::HeaderBar::new());
        view.set_content(Some(&editor));
        view.add_bottom_bar(&bottom_bar);

        dialog.set_child(Some(&view));
        dialog.set_content_width(400);
        dialog.present(self.root().as_ref());
    }
}

#[gtk::template_callbacks]
impl GenericGroupPreferences {
    #[template_callback]
    fn on_add_server_clicked(&self) {
        self.add_server();
    }
}
