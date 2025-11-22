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

use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib;

use libfieldmonitor::connection::ConnectionInstance;
use libfieldmonitor::i18n::gettext_f;

use crate::application::FieldMonitorApplication;
use std::sync::OnceLock;

use glib::subclass::Signal;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorUpdateConnectionDialog)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/update_connection_dialog.ui")]
    pub struct FieldMonitorUpdateConnectionDialog {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub delete_button: TemplateChild<gtk::Button>,
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
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl FieldMonitorUpdateConnectionDialog {
    pub fn new(
        app: &FieldMonitorApplication,
        connection: ConnectionInstance,
        // If a server path is given, this dialog instead edits a single server,
        // not the entire connection.
        server_path: Option<&[String]>,
    ) -> Self {
        // If we are editing a server, we'd ideally show the server's name, otherwise we show
        // the connection name.
        let title = if server_path.is_some() {
            // TODO: Getting the server name here isn't really easy...
            gettext("Edit Server")
        } else {
            gettext_f(
                // Translators: Do NOT translate the content between '{' and '}', this is a
                // variable name.
                "Edit “{title}”",
                &[("title", &connection.title())],
            )
        };

        let slf: Self = glib::Object::builder()
            .property("application", app)
            .property("connection", &connection)
            .property("title", title)
            .build();
        let imp = slf.imp();

        let provider = connection.provider();

        // If we are editing a single server, hide the delete icon, we don't want to be able to
        // delete the connection then.
        // This isn't the most amazing piece of UX, since (at least for the generic group) this
        // means that the edit button that triggers this dialog (the one inside server_row) can't
        // be used to delete the server but when you  edit the entire connection, navigate to the
        // edit of the server there, you suddenly can even though the UI looks identical...?
        //
        // But also, if we were to add an option here to delete a single server this would
        // need so much spaghetti code to support something that isn't even Field Monitor's
        // primary designed use-case, so we just don't.
        if server_path.is_some() {
            imp.delete_button.set_visible(false);
        }

        connection.with_configuration(|configuration| {
            let preferences = provider.preferences(Some(configuration.persistent()), server_path);

            imp.toast_overlay.set_child(Some(&preferences));
            imp.preferences.replace(Some(preferences));
        });

        slf
    }
}

#[gtk::template_callbacks]
impl FieldMonitorUpdateConnectionDialog {
    #[template_callback]
    #[expect(
        clippy::await_holding_refcell_ref,
        reason = "OK because we explicitly drop. See known problems of lint."
    )]
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
                    .timeout(5)
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
