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

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib;

use libfieldmonitor::connection::ConnectionInstance;
use libfieldmonitor::i18n::gettext_f;

use crate::application::FieldMonitorApplication;

mod imp {
    use std::sync::OnceLock;

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorUpdateConnectionDialog)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/update_connection_dialog.ui")]
    pub struct FieldMonitorUpdateConnectionDialog {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        #[property(get, construct_only)]
        pub connection: RefCell<Option<ConnectionInstance>>,
        pub preferences: RefCell<Option<gtk::Widget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorUpdateConnectionDialog {
        const NAME: &'static str = "FieldMonitorUpdateConnectionDialog";
        type Type = super::FieldMonitorUpdateConnectionDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorUpdateConnectionDialog {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("finished-updating").build()])
        }
    }
    impl WidgetImpl for FieldMonitorUpdateConnectionDialog {}
    impl AdwDialogImpl for FieldMonitorUpdateConnectionDialog {}
}

glib::wrapper! {
    pub struct FieldMonitorUpdateConnectionDialog(ObjectSubclass<imp::FieldMonitorUpdateConnectionDialog>)
        @extends gtk::Widget, adw::Dialog;
}

impl FieldMonitorUpdateConnectionDialog {
    pub fn new(app: &FieldMonitorApplication, connection: ConnectionInstance) -> Self {
        let title = connection.title();

        let slf: Self = glib::Object::builder()
            .property("application", app)
            .property("connection", &connection)
            .property("title", gettext_f("Edit {title}", &[("title", &title)]))
            .build();
        let imp = slf.imp();

        let provider = connection.provider();

        connection.with_configuration(|configuration| {
            let preferences = provider.preferences(Some(configuration.persistent()));

            imp.toast_overlay.set_child(Some(&preferences));
            imp.preferences.replace(Some(preferences));
        });

        slf
    }
}

#[gtk::template_callbacks]
impl FieldMonitorUpdateConnectionDialog {
    #[template_callback]
    #[allow(clippy::await_holding_refcell_ref)] // is dropped before
    async fn on_connection_update(&self) {
        let imp = self.imp();
        let connection_brw = imp.connection.borrow();
        let app = imp.application.borrow().clone().unwrap();
        let connection = connection_brw.clone().unwrap();
        let provider = connection.provider();
        let preferences = imp.preferences.borrow().as_ref().cloned().unwrap();
        let old_config = connection_brw
            .as_ref()
            .unwrap()
            .with_configuration(|config| config.explicit_clone());
        drop(connection_brw);

        self.set_can_close(false);
        self.set_sensitive(false);

        match provider.update_connection(preferences, old_config).await {
            Ok(config) => match app.save_connection(config, false).await {
                Ok(_) => {
                    self.emit_by_name::<()>("finished-updating", &[]);
                    self.force_close();
                    return;
                }
                Err(err) => {
                    let alert = adw::AlertDialog::builder()
                        .title(gettext("Failed to save connection"))
                        .body(format!(
                            "{}:\n{}",
                            gettext("An error occurred, while trying to save the connection"),
                            err
                        ))
                        .build();
                    alert.add_response("ok", &gettext("OK"));
                    alert.present(self.parent().as_ref())
                }
            },
            Err(err) => imp.toast_overlay.add_toast(
                adw::Toast::builder()
                    .title(err.to_string())
                    .timeout(10)
                    .build(),
            ),
        }
        self.set_sensitive(true);
        self.set_can_close(true);
    }

    #[template_callback]
    fn on_connection_delete(&self) {
        self.force_close();
        self.application().as_ref().unwrap().activate_action(
            "remove-connection",
            Some(
                &self
                    .connection()
                    .as_ref()
                    .unwrap()
                    .connection_id()
                    .to_variant(),
            ),
        );
    }
}
