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
use adw::ActionRow;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{Orientation, glib};
use libfieldmonitor::cert_security::X509Certificate;
use manual_future::ManualFuture;
use std::cell::RefCell;
use std::rc::Rc;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FieldMonitorCertificateTrustDialog {}

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorCertificateTrustDialog {
        const NAME: &'static str = "FieldMonitorCertificateTrustDialog";
        type Type = super::FieldMonitorCertificateTrustDialog;
        type ParentType = adw::AlertDialog;
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

    pub async fn run_async(
        app: &FieldMonitorApplication,
        cert: &X509Certificate,
        for_host: &str,
    ) -> bool {
        let (fut, completer) = ManualFuture::new();
        let completer = Rc::new(RefCell::new(Some(completer)));

        Self::run_sync(app, cert, for_host, move |_, _, result| {
            let completer = completer.clone();
            glib::spawn_future_local(
                async move { completer.take().unwrap().complete(result).await },
            );
        });

        fut.await
    }

    pub fn run_sync(
        app: &FieldMonitorApplication,
        cert: &X509Certificate,
        for_host: &str,
        callback: impl Fn(&X509Certificate, &str, bool) + 'static,
    ) {
        let dialog = Self::make_dialog(app, cert, for_host);
        let cert = cert.clone();
        let for_host = for_host.to_string();

        dialog.connect_closure(
            "response",
            false,
            glib::closure_local!(move |_: &Self, response: &str| {
                callback(
                    &cert,
                    &for_host,
                    response == FieldMonitorCertificateTrustDialog::RESPONSE_TRUST,
                );
            }),
        );
    }

    fn make_dialog(app: &FieldMonitorApplication, cert: &X509Certificate, for_host: &str) -> Self {
        let slf: Self = glib::Object::builder()
            .property("content-width", 500)
            .property("follows-content-size", false)
            .property("heading", gettext("Trust this server?"))
            .property("body", gettext("The connection to this server is established using a certificate that your system does not automatically trust. Do you trust this server and certificate and want to establish a connection?"))
            .build();

        let boxx = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .build();
        let list_box = gtk::ListBox::builder().css_classes(["boxed-list"]).build();

        let fingerprint = FieldMonitorTrustStore::make_fingerprint_digest(cert).unwrap_or_default();
        // Add zero-width spaces after :
        let fingerprint = fingerprint.replace(':', ":\u{200B}");

        list_box.append(
            &ActionRow::builder()
                .activatable(false)
                .selectable(false)
                .title(gettext("Hostname"))
                .subtitle(for_host)
                .subtitle_selectable(true)
                .css_classes(["property"])
                .build(),
        );
        list_box.append(
            &ActionRow::builder()
                .activatable(false)
                .selectable(false)
                .title(gettext("Fingerprint"))
                .subtitle(fingerprint)
                .subtitle_selectable(true)
                .css_classes(["property", "monospace"])
                .build(),
        );

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

        slf.present(app.active_window().as_ref());

        slf
    }
}
