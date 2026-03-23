/* Copyright 2024-2026 Marco Köpcke
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
use crate::cert_security::FieldMonitorTrustStore;
use crate::widget::certificate_details_window::FieldMonitorCertificateDetailsWindow;
use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::{ActionRow, gio};
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use gtk::{Orientation, glib};
use libfieldmonitor::cert_security::X509Certificate;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FieldMonitorCertificateTrustDialog {
        pub app: RefCell<Option<FieldMonitorApplication>>,
        pub for_host: RefCell<Option<String>>,
        pub cert: RefCell<Option<X509Certificate>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorCertificateTrustDialog {
        const NAME: &'static str = "FieldMonitorCertificateTrustDialog";
        type Type = super::FieldMonitorCertificateTrustDialog;
        type ParentType = adw::AlertDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.install_action(
                "show-certificate",
                None,
                |slf: &super::FieldMonitorCertificateTrustDialog, _, _| {
                    let details_window = FieldMonitorCertificateDetailsWindow::new_from_x509(
                        slf.imp().cert.borrow().as_ref().unwrap(),
                    );
                    details_window.present();
                },
            );
        }
    }

    impl ObjectImpl for FieldMonitorCertificateTrustDialog {}
    impl WidgetImpl for FieldMonitorCertificateTrustDialog {}
    impl AdwDialogImpl for FieldMonitorCertificateTrustDialog {}
    impl AdwAlertDialogImpl for FieldMonitorCertificateTrustDialog {}
}

glib::wrapper! {
    pub struct FieldMonitorCertificateTrustDialog(ObjectSubclass<imp::FieldMonitorCertificateTrustDialog>)
        @extends gtk::Widget, adw::Dialog, adw::AlertDialog,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl FieldMonitorCertificateTrustDialog {
    pub const RESPONSE_TRUST: &'static str = "trust";

    pub fn run_async(
        app: &FieldMonitorApplication,
        cert: &X509Certificate,
        for_host: &str,
    ) -> LocalBoxFuture<'static, bool> {
        let slf = Self::new(app, cert, for_host);
        Box::pin(gio::GioFuture::new(&slf, move |obj, _cancellable, send| {
            let sender = RefCell::new(Some(send));
            obj.run_sync_inner(move |_, _, result| {
                if let Some(sender) = sender.take() {
                    sender.resolve(result);
                }
            });
        }))
    }

    pub fn run_sync(
        app: &FieldMonitorApplication,
        cert: &X509Certificate,
        for_host: &str,
        callback: impl Fn(&X509Certificate, &str, bool) + 'static,
    ) {
        Self::new(app, cert, for_host).run_sync_inner(callback)
    }

    fn run_sync_inner(&self, callback: impl Fn(&X509Certificate, &str, bool) + 'static) {
        self.connect_closure(
            "response",
            false,
            glib::closure_local!(move |slf: &Self, response: &str| {
                callback(
                    slf.imp().cert.borrow().as_ref().unwrap(),
                    slf.imp().for_host.borrow().as_deref().unwrap(),
                    response == FieldMonitorCertificateTrustDialog::RESPONSE_TRUST,
                );
            }),
        );

        self.present(
            self.imp()
                .app
                .borrow()
                .as_ref()
                .unwrap()
                .active_window()
                .as_ref(),
        );
    }

    fn new(app: &FieldMonitorApplication, cert: &X509Certificate, for_host: &str) -> Self {
        let slf: Self = glib::Object::builder()
            .property("content-width", 500)
            .property("follows-content-size", false)
            .property("heading", gettext("Trust this server?"))
            .property("body", gettext("The connection to this server is established using a certificate that your system does not automatically trust. Do you trust this server and certificate and want to establish a connection?"))
            .build();
        slf.imp().app.replace(Some(app.clone()));
        slf.imp().cert.replace(Some(cert.clone()));
        slf.imp().for_host.replace(Some(for_host.to_owned()));

        let boxx = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .build();
        let list_box = gtk::ListBox::builder().css_classes(["boxed-list"]).build();

        let fingerprint = FieldMonitorTrustStore::make_fingerprint_digest(cert).unwrap_or_default();
        // Add zero-width spaces after :
        let fingerprint = fingerprint.replace(':', ":\u{200B}");

        let row_hostname = ActionRow::builder()
            .activatable(false)
            .selectable(false)
            .title(gettext("Hostname"))
            .subtitle(for_host)
            .subtitle_selectable(true)
            .css_classes(["property"])
            .build();
        let row_fingerprint = ActionRow::builder()
            .activatable(false)
            .selectable(false)
            .title(gettext("Fingerprint"))
            .subtitle(fingerprint)
            .subtitle_selectable(true)
            .css_classes(["property", "monospace"])
            .build();

        row_hostname.add_suffix(
            &gtk::Button::builder()
                .action_name("show-certificate")
                .icon_name("application-certificate-symbolic")
                .tooltip_text(gettext("Show Certificate"))
                .css_classes(["flat"])
                .valign(gtk::Align::Center)
                .build(),
        );

        list_box.append(&row_hostname);
        list_box.append(&row_fingerprint);

        boxx.append(&list_box);
        let info = gtk::Label::builder()
            .wrap(true)
            .label(gettext("If you choose to trust this certificate, your choice will be remembered for future connection attempts."))
            .css_classes(["dim-label"])
            .build();
        boxx.append(&info);

        slf.set_extra_child(Some(&boxx));

        slf.add_response("cancel", &gettext("Cancel"));
        slf.add_response(Self::RESPONSE_TRUST, &gettext("Trust Certificate"));
        slf.set_response_appearance(Self::RESPONSE_TRUST, adw::ResponseAppearance::Destructive);
        slf.set_default_response(Some("cancel"));
        slf.set_close_response("cancel");

        slf
    }
}
