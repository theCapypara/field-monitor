//! Minimal `gcr` bindings for Field Monitor, based on code generated with `gir`.

#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]
#![allow(
    clippy::approx_constant,
    clippy::type_complexity,
    clippy::unreadable_literal,
    clippy::upper_case_acronyms
)]

use glib::ffi as glib_ffi;
use glib::gobject_ffi as gobject;
use gtk::gio::ffi as gio;

use libc::size_t;
use std::ffi::{c_char, c_int, c_uint};

use glib_ffi::{gboolean, gpointer, GType};

pub type GcrCertificateSectionFlags = c_uint;
pub const GCR_CERTIFICATE_SECTION_NONE: GcrCertificateSectionFlags = 0;
pub const GCR_CERTIFICATE_SECTION_IMPORTANT: GcrCertificateSectionFlags = 1;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrCertificateFieldClass {
    pub parent_class: gobject::GObjectClass,
}

impl ::std::fmt::Debug for GcrCertificateFieldClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrCertificateFieldClass @ {self:p}"))
            .field("parent_class", &self.parent_class)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrCertificateIface {
    pub parent: gobject::GTypeInterface,
    pub get_der_data: Option<unsafe extern "C" fn(*mut GcrCertificate, *mut size_t) -> *const u8>,
    pub dummy1: gpointer,
    pub dummy2: gpointer,
    pub dummy3: gpointer,
    pub dummy5: gpointer,
    pub dummy6: gpointer,
    pub dummy7: gpointer,
    pub dummy8: gpointer,
}

impl ::std::fmt::Debug for GcrCertificateIface {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrCertificateIface @ {self:p}"))
            .field("parent", &self.parent)
            .field("get_der_data", &self.get_der_data)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrCertificateSectionClass {
    pub parent_class: gobject::GObjectClass,
}

impl ::std::fmt::Debug for GcrCertificateSectionClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrCertificateSectionClass @ {self:p}"))
            .field("parent_class", &self.parent_class)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrSimpleCertificateClass {
    pub parent_class: gobject::GObjectClass,
}

impl ::std::fmt::Debug for GcrSimpleCertificateClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrSimpleCertificateClass @ {self:p}"))
            .field("parent_class", &self.parent_class)
            .finish()
    }
}

#[repr(C)]
pub struct _GcrSimpleCertificatePrivate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub type GcrSimpleCertificatePrivate = _GcrSimpleCertificatePrivate;

#[repr(C)]
pub struct GcrCertificateField {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl ::std::fmt::Debug for GcrCertificateField {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrCertificateField @ {self:p}"))
            .finish()
    }
}

#[repr(C)]
pub struct GcrCertificateSection {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl ::std::fmt::Debug for GcrCertificateSection {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrCertificateSection @ {self:p}"))
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrSimpleCertificate {
    pub parent: gobject::GObject,
    pub pv: *mut GcrSimpleCertificatePrivate,
}

impl ::std::fmt::Debug for GcrSimpleCertificate {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrSimpleCertificate @ {self:p}"))
            .field("parent", &self.parent)
            .finish()
    }
}

// Interfaces
#[repr(C)]
pub struct GcrCertificate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl ::std::fmt::Debug for GcrCertificate {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "GcrCertificate @ {self:p}")
    }
}

unsafe extern "C" {
    //=========================================================================
    // GcrCertificateField
    //=========================================================================
    pub fn gcr_certificate_field_get_type() -> GType;
    pub fn gcr_certificate_field_get_label(self_: *mut GcrCertificateField) -> *const c_char;
    pub fn gcr_certificate_field_get_section(
        self_: *mut GcrCertificateField,
    ) -> *mut GcrCertificateSection;
    pub fn gcr_certificate_field_get_value(
        self_: *mut GcrCertificateField,
        value: *mut gobject::GValue,
    ) -> gboolean;
    pub fn gcr_certificate_field_get_value_type(self_: *mut GcrCertificateField) -> GType;

    //=========================================================================
    // GcrCertificateSection
    //=========================================================================
    pub fn gcr_certificate_section_get_type() -> GType;
    pub fn gcr_certificate_section_get_fields(
        self_: *mut GcrCertificateSection,
    ) -> *mut gio::GListModel;
    pub fn gcr_certificate_section_get_flags(
        self_: *mut GcrCertificateSection,
    ) -> GcrCertificateSectionFlags;
    pub fn gcr_certificate_section_get_label(self_: *mut GcrCertificateSection) -> *const c_char;

    //=========================================================================
    // GcrSimpleCertificate
    //=========================================================================
    pub fn gcr_simple_certificate_get_type() -> GType;
    pub fn gcr_simple_certificate_new(data: *const u8, n_data: size_t)
        -> *mut GcrSimpleCertificate;
    pub fn gcr_simple_certificate_new_static(
        data: *const u8,
        n_data: size_t,
    ) -> *mut GcrSimpleCertificate;

    //=========================================================================
    // GcrCertificate
    //=========================================================================
    pub fn gcr_certificate_get_type() -> GType;
    pub fn gcr_certificate_mixin_class_init(object_class: *mut gobject::GObjectClass);
    pub fn gcr_certificate_mixin_get_property(
        obj: *mut gobject::GObject,
        prop_id: c_uint,
        value: *mut gobject::GValue,
        pspec: *mut gobject::GParamSpec,
    );
    pub fn gcr_certificate_get_basic_constraints(
        self_: *mut GcrCertificate,
        is_ca: *mut gboolean,
        path_len: *mut c_int,
    ) -> gboolean;
    pub fn gcr_certificate_get_der_data(
        self_: *mut GcrCertificate,
        n_data: *mut size_t,
    ) -> *const u8;
    pub fn gcr_certificate_get_expiry_date(self_: *mut GcrCertificate) -> *mut glib_ffi::GDateTime;
    pub fn gcr_certificate_get_fingerprint(
        self_: *mut GcrCertificate,
        type_: glib_ffi::GChecksumType,
        n_length: *mut size_t,
    ) -> *mut u8;
    pub fn gcr_certificate_get_fingerprint_hex(
        self_: *mut GcrCertificate,
        type_: glib_ffi::GChecksumType,
    ) -> *mut c_char;
    pub fn gcr_certificate_get_interface_elements(
        self_: *mut GcrCertificate,
    ) -> *mut glib_ffi::GList;
    pub fn gcr_certificate_get_issued_date(self_: *mut GcrCertificate) -> *mut glib_ffi::GDateTime;
    pub fn gcr_certificate_get_issuer_cn(self_: *mut GcrCertificate) -> *mut c_char;
    pub fn gcr_certificate_get_issuer_dn(self_: *mut GcrCertificate) -> *mut c_char;
    pub fn gcr_certificate_get_issuer_name(self_: *mut GcrCertificate) -> *mut c_char;
    pub fn gcr_certificate_get_issuer_part(
        self_: *mut GcrCertificate,
        part: *const c_char,
    ) -> *mut c_char;
    pub fn gcr_certificate_get_issuer_raw(
        self_: *mut GcrCertificate,
        n_data: *mut size_t,
    ) -> *mut u8;
    pub fn gcr_certificate_get_key_size(self_: *mut GcrCertificate) -> c_uint;
    pub fn gcr_certificate_get_serial_number(
        self_: *mut GcrCertificate,
        n_length: *mut size_t,
    ) -> *mut u8;
    pub fn gcr_certificate_get_serial_number_hex(self_: *mut GcrCertificate) -> *mut c_char;
    pub fn gcr_certificate_get_subject_cn(self_: *mut GcrCertificate) -> *mut c_char;
    pub fn gcr_certificate_get_subject_dn(self_: *mut GcrCertificate) -> *mut c_char;
    pub fn gcr_certificate_get_subject_name(self_: *mut GcrCertificate) -> *mut c_char;
    pub fn gcr_certificate_get_subject_part(
        self_: *mut GcrCertificate,
        part: *const c_char,
    ) -> *mut c_char;
    pub fn gcr_certificate_get_subject_raw(
        self_: *mut GcrCertificate,
        n_data: *mut size_t,
    ) -> *mut u8;
    pub fn gcr_certificate_is_issuer(
        self_: *mut GcrCertificate,
        issuer: *mut GcrCertificate,
    ) -> gboolean;
    pub fn gcr_certificate_mixin_emit_notify(self_: *mut GcrCertificate);
}
