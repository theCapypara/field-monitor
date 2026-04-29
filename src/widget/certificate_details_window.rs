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

//! Certificate details dialog using `gcr`.

use self::gcr::*;
use crate::cert_security::format_bytes_as_hex_string;
use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::{ActionRow, gio};
use gettextrs::gettext;
use libfieldmonitor::cert_security::{Encode, X509Certificate};
use log::warn;
use std::cell::RefCell;
use std::fmt::Display;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorCertificateDetailsWindow)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/certificate_details_window.ui")]
    pub struct FieldMonitorCertificateDetailsWindow {
        #[template_child]
        pub content: TemplateChild<adw::PreferencesPage>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[property(get, construct_only)]
        pub certificate: RefCell<Option<SimpleCertificate>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorCertificateDetailsWindow {
        const NAME: &'static str = "FieldMonitorCertificateDetailsWindow";
        type Type = super::FieldMonitorCertificateDetailsWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(
                "certificate.export",
                None,
                |slf: &super::FieldMonitorCertificateDetailsWindow, _, _| {
                    glib::spawn_future_local(glib::clone!(
                        #[strong]
                        slf,
                        async move {
                            slf.export_certificate().await;
                        }
                    ));
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorCertificateDetailsWindow {}
    impl WidgetImpl for FieldMonitorCertificateDetailsWindow {}
    impl WindowImpl for FieldMonitorCertificateDetailsWindow {}
    impl AdwWindowImpl for FieldMonitorCertificateDetailsWindow {}
}

glib::wrapper! {
    pub struct FieldMonitorCertificateDetailsWindow(ObjectSubclass<imp::FieldMonitorCertificateDetailsWindow>)
        @extends gtk::Widget, adw::Window, gtk::Window,
        @implements gtk::ShortcutManager, gtk::Root, gtk::Native, gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl FieldMonitorCertificateDetailsWindow {
    pub fn new(cert: &SimpleCertificate) -> Self {
        let slf: Self = glib::Object::builder()
            .property("certificate", cert.clone())
            .build();

        let page = slf.imp().content.get();
        let sections = cert.upcast_ref::<Certificate>().get_interface_elements();

        let mut important_groups: Vec<adw::PreferencesGroup> = Vec::new();
        let mut other_groups: Vec<adw::PreferencesGroup> = Vec::new();

        for section in sections {
            let target = if section
                .flags()
                .intersects(CertificateSectionFlags::IMPORTANT)
            {
                &mut important_groups
            } else {
                &mut other_groups
            };
            let group = adw::PreferencesGroup::builder()
                .title(glib::markup_escape_text(&section.label()))
                .build();

            for field in section.fields().iter::<CertificateField>() {
                let field = field.unwrap();
                let value = field.value().unwrap();

                let css_classes = if value.type_().name() == "GBytes" {
                    static MONOSPACE_PROPERTY: [&str; 2] = ["property", "monospace"];
                    &MONOSPACE_PROPERTY[..]
                } else {
                    static PROPERTY: [&str; 1] = ["property"];
                    &PROPERTY[..]
                };

                let row = ActionRow::builder()
                    .activatable(false)
                    .selectable(false)
                    .title(glib::markup_escape_text(&field.label()))
                    .subtitle(glib::markup_escape_text(
                        &value_as_gstr(value)
                            .unwrap_or_else(|| "< failed to display value >".into()),
                    ))
                    .subtitle_selectable(true)
                    .css_classes(css_classes)
                    .build();

                group.add(&row);
            }

            target.push(group);
        }

        for group in important_groups.into_iter().chain(other_groups) {
            page.add(&group);
        }

        slf
    }
    pub fn new_from_x509(cert: &X509Certificate) -> Self {
        Self::new(&SimpleCertificate::from(&cert.to_der().unwrap()))
    }

    async fn export_certificate(&self) {
        let filters = gio::ListStore::new::<gtk::FileFilter>();

        let pem_filter = gtk::FileFilter::new();
        pem_filter.add_suffix("pem");
        pem_filter.add_mime_type("application/x-pem-file");
        pem_filter.set_name(Some(&gettext("PEM Certificate")));

        let der_filter = gtk::FileFilter::new();
        der_filter.add_suffix("der");
        der_filter.add_mime_type("application/x-x509-ca-cert");
        der_filter.set_name(Some(&gettext("DER Certificate")));

        filters.append(&pem_filter);
        filters.append(&der_filter);

        let file_dialog = gtk::FileDialog::builder()
            .title(gettext("Export Certificate..."))
            .filters(&filters)
            .build();
        let result = file_dialog.save_future(Some(self)).await;
        match result {
            Ok(file) => match self.write_certificate(file).await {
                Ok(_) => self
                    .imp()
                    .toast_overlay
                    .add_toast(adw::Toast::builder().title("Certificate exported.").build()),
                Err(err) => failed_to_save(err, self),
            },
            Err(err) => match err.kind::<gtk::DialogError>() {
                Some(gtk::DialogError::Cancelled) | Some(gtk::DialogError::Dismissed) => {}
                _ => failed_to_save(err, self),
            },
        }
    }

    async fn write_certificate(&self, file: gio::File) -> Result<(), anyhow::Error> {
        let cert = self.imp().certificate.borrow().clone().unwrap();
        let cert_up = cert.upcast_ref::<Certificate>();
        let der_data = unsafe { cert_up.der_data() };
        let file_ext = file
            .path()
            .as_ref()
            .unwrap()
            .extension()
            .unwrap_or_default()
            .to_ascii_lowercase();
        dbg!(&file_ext);

        let contents = if file_ext == "der" {
            der_data.to_vec()
        } else {
            let pem = pem::Pem::new("CERTIFICATE", der_data);
            pem.to_string().into_bytes()
        };

        let rw = file
            .replace_readwrite_future(
                None,
                false,
                gio::FileCreateFlags::NONE,
                glib::Priority::DEFAULT,
            )
            .await?;
        let stream = rw.output_stream();
        stream
            .write_all_future(contents, glib::Priority::DEFAULT)
            .await
            .map_err(|(_, err)| err)?;
        Ok(())
    }
}

fn failed_to_save(err: impl Display, window: &impl IsA<gtk::Window>) {
    warn!("Failed to save file: {err}");
    let alert = adw::AlertDialog::builder()
        .heading(gettext("Failed to save"))
        .body(gettext(
            "Field Monitor was unable to save the provided file.",
        ))
        .build();
    alert.add_response("ok", &gettext("OK"));
    alert.present(Some(window.upcast_ref()));
}

fn value_as_gstr(value: glib::Value) -> Option<glib::GString> {
    match value.type_().name() {
        "gchararray" => value.get::<glib::GString>().ok(),
        "GBytes" => {
            let v = value.get::<glib::Bytes>().ok()?;
            let fingerprint = format_bytes_as_hex_string(v.as_ref());
            // Add zero-width spaces after :
            let fingerprint = fingerprint.replace(':', ":\u{200B}");
            Some(fingerprint.into())
        }
        _ => None,
    }
}

/// Minimal Rust wrappers for `gcr`.
mod gcr {
    use gcr_sys_minimal as ffi;
    use glib::translate::{
        FromGlib, FromGlibPtrContainer, IntoGlib, ToGlibPtr, ToGlibPtrMut, from_glib,
        from_glib_full, from_glib_none, mut_override,
    };
    use gtk::gio;
    use std::borrow::Borrow;
    use std::slice;

    glib::bitflags::bitflags! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct CertificateSectionFlags: u32 {
            const NONE = ffi::GCR_CERTIFICATE_SECTION_NONE as _;
            const IMPORTANT = ffi::GCR_CERTIFICATE_SECTION_IMPORTANT as _;
        }
    }

    impl IntoGlib for CertificateSectionFlags {
        type GlibType = ffi::GcrCertificateSectionFlags;

        #[inline]
        fn into_glib(self) -> ffi::GcrCertificateSectionFlags {
            self.bits()
        }
    }

    impl FromGlib<ffi::GcrCertificateSectionFlags> for CertificateSectionFlags {
        #[inline]
        unsafe fn from_glib(value: ffi::GcrCertificateSectionFlags) -> Self {
            Self::from_bits_truncate(value)
        }
    }

    glib::wrapper! {
        pub struct CertificateField(Object<ffi::GcrCertificateField>);

        match fn {
            type_ => || ffi::gcr_certificate_field_get_type(),
        }
    }

    impl CertificateField {
        pub fn label(&self) -> glib::GString {
            unsafe { from_glib_none(ffi::gcr_certificate_field_get_label(self.to_glib_none().0)) }
        }
        pub fn value(&self) -> Option<glib::Value> {
            unsafe {
                let type_: glib::Type = from_glib(ffi::gcr_certificate_field_get_value_type(
                    self.to_glib_none().0,
                ));
                let mut value = glib::Value::from_type(type_);
                let ok = ffi::gcr_certificate_field_get_value(
                    self.to_glib_none().0,
                    value.to_glib_none_mut().0,
                );
                if ok > 0 { Some(value) } else { None }
            }
        }
    }

    glib::wrapper! {
        pub struct CertificateSection(Object<ffi::GcrCertificateSection>);

        match fn {
            type_ => || ffi::gcr_certificate_section_get_type(),
        }
    }

    impl CertificateSection {
        pub fn fields(&self) -> gio::ListModel {
            unsafe {
                from_glib_none(ffi::gcr_certificate_section_get_fields(
                    self.to_glib_none().0,
                ))
            }
        }
        pub fn flags(&self) -> CertificateSectionFlags {
            unsafe {
                from_glib(ffi::gcr_certificate_section_get_flags(
                    self.to_glib_none().0,
                ))
            }
        }
        pub fn label(&self) -> glib::GString {
            unsafe {
                from_glib_none(ffi::gcr_certificate_section_get_label(
                    self.to_glib_none().0,
                ))
            }
        }
    }

    glib::wrapper! {
        pub struct Certificate(Interface<ffi::GcrCertificate, ffi::GcrCertificateIface>);

        match fn {
            type_ => || ffi::gcr_certificate_get_type(),
        }
    }

    impl Certificate {
        pub fn get_interface_elements(&self) -> Vec<CertificateSection> {
            unsafe {
                FromGlibPtrContainer::from_glib_container(
                    ffi::gcr_certificate_get_interface_elements(mut_override(
                        self.to_glib_none().0,
                    )),
                )
            }
        }
        // SAFETY: This is only safe to call if the caller makes 100% sure that the object is not destroyed
        // and the certificate buffer isn't modified by any means.
        pub unsafe fn der_data(&self) -> &[u8] {
            unsafe {
                let mut out_size: glib::ffi::gsize = 0;
                let slice_ptr =
                    ffi::gcr_certificate_get_der_data(self.to_glib_none().0, &mut out_size);
                slice::from_raw_parts(slice_ptr, out_size)
            }
        }
    }

    glib::wrapper! {
        pub struct SimpleCertificate(Object<ffi::GcrSimpleCertificate>) @implements Certificate;

        match fn {
            type_ => || ffi::gcr_simple_certificate_get_type(),
        }
    }

    impl SimpleCertificate {
        pub fn new<'a, T: ?Sized + Borrow<[u8]> + 'a>(data: &'a T) -> Self {
            SimpleCertificate::from(data)
        }
    }

    impl<'a, T: ?Sized + Borrow<[u8]> + 'a> From<&'a T> for SimpleCertificate {
        fn from(value: &'a T) -> SimpleCertificate {
            let value = value.borrow();
            unsafe {
                let obj = ffi::gcr_simple_certificate_new(value.as_ptr(), value.len());
                from_glib_full(obj)
            }
        }
    }
}
