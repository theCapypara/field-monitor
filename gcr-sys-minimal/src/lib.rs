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
/*
// Enums
pub type GcrCertificateChainStatus = c_int;
pub const GCR_CERTIFICATE_CHAIN_UNKNOWN: GcrCertificateChainStatus = 0;
pub const GCR_CERTIFICATE_CHAIN_INCOMPLETE: GcrCertificateChainStatus = 1;
pub const GCR_CERTIFICATE_CHAIN_DISTRUSTED: GcrCertificateChainStatus = 2;
pub const GCR_CERTIFICATE_CHAIN_SELFSIGNED: GcrCertificateChainStatus = 3;
pub const GCR_CERTIFICATE_CHAIN_PINNED: GcrCertificateChainStatus = 4;
pub const GCR_CERTIFICATE_CHAIN_ANCHORED: GcrCertificateChainStatus = 5;

pub type GcrCertificateRequestFormat = c_int;
pub const GCR_CERTIFICATE_REQUEST_PKCS10: GcrCertificateRequestFormat = 1;

pub type GcrDataError = c_int;
pub const GCR_ERROR_FAILURE: GcrDataError = -1;
pub const GCR_ERROR_UNRECOGNIZED: GcrDataError = 1;
pub const GCR_ERROR_CANCELLED: GcrDataError = 2;
pub const GCR_ERROR_LOCKED: GcrDataError = 3;

pub type GcrDataFormat = c_int;
pub const GCR_FORMAT_ALL: GcrDataFormat = -1;
pub const GCR_FORMAT_INVALID: GcrDataFormat = 0;
pub const GCR_FORMAT_DER_PRIVATE_KEY: GcrDataFormat = 100;
pub const GCR_FORMAT_DER_PRIVATE_KEY_RSA: GcrDataFormat = 101;
pub const GCR_FORMAT_DER_PRIVATE_KEY_DSA: GcrDataFormat = 102;
pub const GCR_FORMAT_DER_PRIVATE_KEY_EC: GcrDataFormat = 103;
pub const GCR_FORMAT_DER_SUBJECT_PUBLIC_KEY: GcrDataFormat = 150;
pub const GCR_FORMAT_DER_CERTIFICATE_X509: GcrDataFormat = 200;
pub const GCR_FORMAT_DER_PKCS7: GcrDataFormat = 300;
pub const GCR_FORMAT_DER_PKCS8: GcrDataFormat = 400;
pub const GCR_FORMAT_DER_PKCS8_PLAIN: GcrDataFormat = 401;
pub const GCR_FORMAT_DER_PKCS8_ENCRYPTED: GcrDataFormat = 402;
pub const GCR_FORMAT_DER_PKCS10: GcrDataFormat = 450;
pub const GCR_FORMAT_DER_SPKAC: GcrDataFormat = 455;
pub const GCR_FORMAT_BASE64_SPKAC: GcrDataFormat = 456;
pub const GCR_FORMAT_DER_PKCS12: GcrDataFormat = 500;
pub const GCR_FORMAT_OPENSSH_PUBLIC: GcrDataFormat = 600;
pub const GCR_FORMAT_OPENPGP_PACKET: GcrDataFormat = 700;
pub const GCR_FORMAT_OPENPGP_ARMOR: GcrDataFormat = 701;
pub const GCR_FORMAT_PEM: GcrDataFormat = 1000;
pub const GCR_FORMAT_PEM_PRIVATE_KEY_RSA: GcrDataFormat = 1001;
pub const GCR_FORMAT_PEM_PRIVATE_KEY_DSA: GcrDataFormat = 1002;
pub const GCR_FORMAT_PEM_CERTIFICATE_X509: GcrDataFormat = 1003;
pub const GCR_FORMAT_PEM_PKCS7: GcrDataFormat = 1004;
pub const GCR_FORMAT_PEM_PKCS8_PLAIN: GcrDataFormat = 1005;
pub const GCR_FORMAT_PEM_PKCS8_ENCRYPTED: GcrDataFormat = 1006;
pub const GCR_FORMAT_PEM_PKCS12: GcrDataFormat = 1007;
pub const GCR_FORMAT_PEM_PRIVATE_KEY: GcrDataFormat = 1008;
pub const GCR_FORMAT_PEM_PKCS10: GcrDataFormat = 1009;
pub const GCR_FORMAT_PEM_PRIVATE_KEY_EC: GcrDataFormat = 1010;
pub const GCR_FORMAT_PEM_PUBLIC_KEY: GcrDataFormat = 1011;

pub type GcrPromptReply = c_int;
pub const GCR_PROMPT_REPLY_CANCEL: GcrPromptReply = 0;
pub const GCR_PROMPT_REPLY_CONTINUE: GcrPromptReply = 1;

pub type GcrSystemPromptError = c_int;
pub const GCR_SYSTEM_PROMPT_IN_PROGRESS: GcrSystemPromptError = 1;

pub type GcrSystemPrompterMode = c_int;
pub const GCR_SYSTEM_PROMPTER_SINGLE: GcrSystemPrompterMode = 0;
pub const GCR_SYSTEM_PROMPTER_MULTIPLE: GcrSystemPrompterMode = 1;

// Constants
pub const GCR_MAJOR_VERSION: c_int = 4;
pub const GCR_MICRO_VERSION: c_int = 0;
pub const GCR_MINOR_VERSION: c_int = 2;
pub const GCR_PURPOSE_CLIENT_AUTH: &[u8] = b"1.3.6.1.5.5.7.3.2\0";
pub const GCR_PURPOSE_CODE_SIGNING: &[u8] = b"1.3.6.1.5.5.7.3.3\0";
pub const GCR_PURPOSE_EMAIL: &[u8] = b"1.3.6.1.5.5.7.3.4\0";
pub const GCR_PURPOSE_SERVER_AUTH: &[u8] = b"1.3.6.1.5.5.7.3.1\0";
pub const GCR_SECRET_EXCHANGE_PROTOCOL_1: &[u8] = b"sx-aes-1\0";
pub const GCR_UNLOCK_OPTION_ALWAYS: &[u8] = b"always\0";
pub const GCR_UNLOCK_OPTION_IDLE: &[u8] = b"idle\0";
pub const GCR_UNLOCK_OPTION_SESSION: &[u8] = b"session\0";
pub const GCR_UNLOCK_OPTION_TIMEOUT: &[u8] = b"timeout\0";

// Flags
pub type GcrCertificateChainFlags = c_uint;
pub const GCR_CERTIFICATE_CHAIN_NONE: GcrCertificateChainFlags = 0;
pub const GCR_CERTIFICATE_CHAIN_NO_LOOKUPS: GcrCertificateChainFlags = 1;
*/
pub type GcrCertificateSectionFlags = c_uint;
pub const GCR_CERTIFICATE_SECTION_NONE: GcrCertificateSectionFlags = 0;
pub const GCR_CERTIFICATE_SECTION_IMPORTANT: GcrCertificateSectionFlags = 1;
/*
// Records
#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrCertificateChainClass {
    pub parent_class: gobject::GObjectClass,
}

impl ::std::fmt::Debug for GcrCertificateChainClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrCertificateChainClass @ {self:p}"))
            .field("parent_class", &self.parent_class)
            .finish()
    }
}

#[repr(C)]
pub struct _GcrCertificateChainPrivate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub type GcrCertificateChainPrivate = _GcrCertificateChainPrivate;
*/
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

/*
#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrCertificateRequestClass {
    pub parent_class: gobject::GObjectClass,
}

impl ::std::fmt::Debug for GcrCertificateRequestClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrCertificateRequestClass @ {self:p}"))
            .field("parent_class", &self.parent_class)
            .finish()
    }
}
*/
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
/*
#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrImportInteractionInterface {
    pub parent: gobject::GTypeInterface,
    pub supplement_prep:
        Option<unsafe extern "C" fn(*mut GcrImportInteraction, *mut gck::GckBuilder)>,
    pub supplement: Option<
        unsafe extern "C" fn(
            *mut GcrImportInteraction,
            *mut gck::GckBuilder,
            *mut gio::GCancellable,
            *mut *mut glib_ffi::GError,
        ) -> gio::GTlsInteractionResult,
    >,
    pub supplement_async: Option<
        unsafe extern "C" fn(
            *mut GcrImportInteraction,
            *mut gck::GckBuilder,
            *mut gio::GCancellable,
            gio::GAsyncReadyCallback,
            gpointer,
        ),
    >,
    pub supplement_finish: Option<
        unsafe extern "C" fn(
            *mut GcrImportInteraction,
            *mut gio::GAsyncResult,
            *mut *mut glib_ffi::GError,
        ) -> gio::GTlsInteractionResult,
    >,
    pub reserved: [gpointer; 6],
}

impl ::std::fmt::Debug for GcrImportInteractionInterface {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrImportInteractionInterface @ {self:p}"))
            .field("parent", &self.parent)
            .field("supplement_prep", &self.supplement_prep)
            .field("supplement", &self.supplement)
            .field("supplement_async", &self.supplement_async)
            .field("supplement_finish", &self.supplement_finish)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrImporterInterface {
    pub parent: gobject::GTypeInterface,
    pub create_for_parsed: Option<unsafe extern "C" fn(*mut GcrParsed) -> *mut glib_ffi::GList>,
    pub queue_for_parsed:
        Option<unsafe extern "C" fn(*mut GcrImporter, *mut GcrParsed) -> gboolean>,
    pub import_async: Option<
        unsafe extern "C" fn(
            *mut GcrImporter,
            *mut gio::GCancellable,
            gio::GAsyncReadyCallback,
            gpointer,
        ),
    >,
    pub import_finish: Option<
        unsafe extern "C" fn(
            *mut GcrImporter,
            *mut gio::GAsyncResult,
            *mut *mut glib_ffi::GError,
        ) -> gboolean,
    >,
    pub reserved: [gpointer; 14],
}

impl ::std::fmt::Debug for GcrImporterInterface {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrImporterInterface @ {self:p}"))
            .field("parent", &self.parent)
            .field("create_for_parsed", &self.create_for_parsed)
            .field("queue_for_parsed", &self.queue_for_parsed)
            .field("import_async", &self.import_async)
            .field("import_finish", &self.import_finish)
            .finish()
    }
}

#[repr(C)]
pub struct GcrParsed {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl ::std::fmt::Debug for GcrParsed {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrParsed @ {self:p}")).finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrParserClass {
    pub parent_class: gobject::GObjectClass,
    pub authenticate: Option<unsafe extern "C" fn(*mut GcrParser, c_int) -> gboolean>,
    pub parsed: Option<unsafe extern "C" fn(*mut GcrParser)>,
}

impl ::std::fmt::Debug for GcrParserClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrParserClass @ {self:p}"))
            .field("parent_class", &self.parent_class)
            .field("authenticate", &self.authenticate)
            .field("parsed", &self.parsed)
            .finish()
    }
}

#[repr(C)]
pub struct _GcrParserPrivate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub type GcrParserPrivate = _GcrParserPrivate;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrPkcs11CertificateClass {
    pub parent_class: gck::GckObjectClass,
}

impl ::std::fmt::Debug for GcrPkcs11CertificateClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrPkcs11CertificateClass @ {self:p}"))
            .finish()
    }
}

#[repr(C)]
pub struct _GcrPkcs11CertificatePrivate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub type GcrPkcs11CertificatePrivate = _GcrPkcs11CertificatePrivate;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrPromptInterface {
    pub parent_iface: gobject::GTypeInterface,
    pub prompt_password_async: Option<
        unsafe extern "C" fn(
            *mut GcrPrompt,
            *mut gio::GCancellable,
            gio::GAsyncReadyCallback,
            gpointer,
        ),
    >,
    pub prompt_password_finish: Option<
        unsafe extern "C" fn(
            *mut GcrPrompt,
            *mut gio::GAsyncResult,
            *mut *mut glib_ffi::GError,
        ) -> *const c_char,
    >,
    pub prompt_confirm_async: Option<
        unsafe extern "C" fn(
            *mut GcrPrompt,
            *mut gio::GCancellable,
            gio::GAsyncReadyCallback,
            gpointer,
        ),
    >,
    pub prompt_confirm_finish: Option<
        unsafe extern "C" fn(
            *mut GcrPrompt,
            *mut gio::GAsyncResult,
            *mut *mut glib_ffi::GError,
        ) -> GcrPromptReply,
    >,
    pub prompt_close: Option<unsafe extern "C" fn(*mut GcrPrompt)>,
}

impl ::std::fmt::Debug for GcrPromptInterface {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrPromptInterface @ {self:p}"))
            .field("parent_iface", &self.parent_iface)
            .field("prompt_password_async", &self.prompt_password_async)
            .field("prompt_password_finish", &self.prompt_password_finish)
            .field("prompt_confirm_async", &self.prompt_confirm_async)
            .field("prompt_confirm_finish", &self.prompt_confirm_finish)
            .field("prompt_close", &self.prompt_close)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrSecretExchangeClass {
    pub parent_class: gobject::GObjectClass,
    pub generate_exchange_key: Option<
        unsafe extern "C" fn(
            *mut GcrSecretExchange,
            *const c_char,
            *mut *mut u8,
            *mut size_t,
        ) -> gboolean,
    >,
    pub derive_transport_key:
        Option<unsafe extern "C" fn(*mut GcrSecretExchange, *const u8, size_t) -> gboolean>,
    pub encrypt_transport_data: Option<
        unsafe extern "C" fn(
            *mut GcrSecretExchange,
            gck::GckAllocator,
            *const u8,
            size_t,
            *const u8,
            size_t,
            *mut *mut u8,
            *mut size_t,
        ) -> gboolean,
    >,
    pub decrypt_transport_data: Option<
        unsafe extern "C" fn(
            *mut GcrSecretExchange,
            gck::GckAllocator,
            *const u8,
            size_t,
            *const u8,
            size_t,
            *mut *mut u8,
            *mut size_t,
        ) -> gboolean,
    >,
    pub dummy: [gpointer; 6],
}

impl ::std::fmt::Debug for GcrSecretExchangeClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrSecretExchangeClass @ {self:p}"))
            .field("generate_exchange_key", &self.generate_exchange_key)
            .field("derive_transport_key", &self.derive_transport_key)
            .field("encrypt_transport_data", &self.encrypt_transport_data)
            .field("decrypt_transport_data", &self.decrypt_transport_data)
            .finish()
    }
}

#[repr(C)]
pub struct _GcrSecretExchangePrivate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub type GcrSecretExchangePrivate = _GcrSecretExchangePrivate;
*/
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
/*
#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrSshAskpassClass {
    pub parent_class: gobject::GObjectClass,
}

impl ::std::fmt::Debug for GcrSshAskpassClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrSshAskpassClass @ {self:p}"))
            .field("parent_class", &self.parent_class)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrSystemPromptClass {
    pub parent_class: gobject::GObjectClass,
}

impl ::std::fmt::Debug for GcrSystemPromptClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrSystemPromptClass @ {self:p}"))
            .field("parent_class", &self.parent_class)
            .finish()
    }
}

#[repr(C)]
pub struct _GcrSystemPromptPrivate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub type GcrSystemPromptPrivate = _GcrSystemPromptPrivate;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrSystemPrompterClass {
    pub parent_class: gobject::GObjectClass,
    pub new_prompt: Option<unsafe extern "C" fn(*mut GcrSystemPrompter) -> *mut GcrPrompt>,
    pub padding: [gpointer; 7],
}

impl ::std::fmt::Debug for GcrSystemPrompterClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrSystemPrompterClass @ {self:p}"))
            .field("parent_class", &self.parent_class)
            .field("new_prompt", &self.new_prompt)
            .finish()
    }
}

#[repr(C)]
pub struct _GcrSystemPrompterPrivate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub type GcrSystemPrompterPrivate = _GcrSystemPrompterPrivate;

// Classes
#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrCertificateChain {
    pub parent: gobject::GObject,
    pub pv: *mut GcrCertificateChainPrivate,
}

impl ::std::fmt::Debug for GcrCertificateChain {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrCertificateChain @ {self:p}"))
            .field("parent", &self.parent)
            .finish()
    }
}
*/
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
/*
#[repr(C)]
pub struct GcrCertificateRequest {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl ::std::fmt::Debug for GcrCertificateRequest {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrCertificateRequest @ {self:p}"))
            .finish()
    }
}
*/
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
/*
#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrParser {
    pub parent: gobject::GObject,
    pub pv: *mut GcrParserPrivate,
}

impl ::std::fmt::Debug for GcrParser {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrParser @ {self:p}"))
            .field("parent", &self.parent)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrPkcs11Certificate {
    pub parent: gck::GckObject,
    pub pv: *mut GcrPkcs11CertificatePrivate,
}

impl ::std::fmt::Debug for GcrPkcs11Certificate {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrPkcs11Certificate @ {self:p}"))
            .field("parent", &self.parent)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrSecretExchange {
    pub parent: gobject::GObject,
    pub pv: *mut GcrSecretExchangePrivate,
}

impl ::std::fmt::Debug for GcrSecretExchange {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrSecretExchange @ {self:p}"))
            .finish()
    }
}
*/
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
/*
#[repr(C)]
pub struct GcrSshAskpass {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl ::std::fmt::Debug for GcrSshAskpass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrSshAskpass @ {self:p}"))
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrSystemPrompt {
    pub parent: gobject::GObject,
    pub pv: *mut GcrSystemPromptPrivate,
}

impl ::std::fmt::Debug for GcrSystemPrompt {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrSystemPrompt @ {self:p}"))
            .field("parent", &self.parent)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GcrSystemPrompter {
    pub parent: gobject::GObject,
    pub pv: *mut GcrSystemPrompterPrivate,
}

impl ::std::fmt::Debug for GcrSystemPrompter {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("GcrSystemPrompter @ {self:p}"))
            .field("parent", &self.parent)
            .finish()
    }
}
*/
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
/*
#[repr(C)]
pub struct GcrImportInteraction {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl ::std::fmt::Debug for GcrImportInteraction {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "GcrImportInteraction @ {self:p}")
    }
}

#[repr(C)]
pub struct GcrImporter {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl ::std::fmt::Debug for GcrImporter {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "GcrImporter @ {self:p}")
    }
}

#[repr(C)]
pub struct GcrPrompt {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl ::std::fmt::Debug for GcrPrompt {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "GcrPrompt @ {self:p}")
    }
}
*/
unsafe extern "C" {
    /*
                        //=========================================================================
                        // GcrParsed
                        //=========================================================================
                        pub fn gcr_parsed_get_type() -> GType;
                        pub fn gcr_parsed_get_attributes(parsed: *mut GcrParsed) -> *mut gck::GckAttributes;
                        pub fn gcr_parsed_get_bytes(parsed: *mut GcrParsed) -> *mut glib_ffi::GBytes;
                        pub fn gcr_parsed_get_data(parsed: *mut GcrParsed, n_data: *mut size_t) -> *const u8;
                        pub fn gcr_parsed_get_description(parsed: *mut GcrParsed) -> *const c_char;
                        pub fn gcr_parsed_get_filename(parsed: *mut GcrParsed) -> *const c_char;
                        pub fn gcr_parsed_get_format(parsed: *mut GcrParsed) -> GcrDataFormat;
                        pub fn gcr_parsed_get_label(parsed: *mut GcrParsed) -> *const c_char;
                        pub fn gcr_parsed_ref(parsed: *mut GcrParsed) -> *mut GcrParsed;
                        pub fn gcr_parsed_unref(parsed: gpointer);

                        //=========================================================================
                        // GcrCertificateChain
                        //=========================================================================
                        pub fn gcr_certificate_chain_get_type() -> GType;
                        pub fn gcr_certificate_chain_new() -> *mut GcrCertificateChain;
                        pub fn gcr_certificate_chain_add(
                            self_: *mut GcrCertificateChain,
                            certificate: *mut GcrCertificate,
                        );
                        pub fn gcr_certificate_chain_build(
                            self_: *mut GcrCertificateChain,
                            purpose: *const c_char,
                            peer: *const c_char,
                            flags: GcrCertificateChainFlags,
                            cancellable: *mut gio::GCancellable,
                            error: *mut *mut glib_ffi::GError,
                        ) -> gboolean;
                        pub fn gcr_certificate_chain_build_async(
                            self_: *mut GcrCertificateChain,
                            purpose: *const c_char,
                            peer: *const c_char,
                            flags: GcrCertificateChainFlags,
                            cancellable: *mut gio::GCancellable,
                            callback: gio::GAsyncReadyCallback,
                            user_data: gpointer,
                        );
                        pub fn gcr_certificate_chain_build_finish(
                            self_: *mut GcrCertificateChain,
                            result: *mut gio::GAsyncResult,
                            error: *mut *mut glib_ffi::GError,
                        ) -> gboolean;
                        pub fn gcr_certificate_chain_get_anchor(self_: *mut GcrCertificateChain)
                            -> *mut GcrCertificate;
                        pub fn gcr_certificate_chain_get_certificate(
                            self_: *mut GcrCertificateChain,
                            index: c_uint,
                        ) -> *mut GcrCertificate;
                        pub fn gcr_certificate_chain_get_endpoint(
                            self_: *mut GcrCertificateChain,
                        ) -> *mut GcrCertificate;
                        pub fn gcr_certificate_chain_get_length(self_: *mut GcrCertificateChain) -> c_uint;
                        pub fn gcr_certificate_chain_get_status(
                            self_: *mut GcrCertificateChain,
                        ) -> GcrCertificateChainStatus;
    */
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
    /*
                    //=========================================================================
                    // GcrCertificateRequest
                    //=========================================================================
                    pub fn gcr_certificate_request_get_type() -> GType;
                    pub fn gcr_certificate_request_capable(
                        private_key: *mut gck::GckObject,
                        cancellable: *mut gio::GCancellable,
                        error: *mut *mut glib_ffi::GError,
                    ) -> gboolean;
                    pub fn gcr_certificate_request_capable_async(
                        private_key: *mut gck::GckObject,
                        cancellable: *mut gio::GCancellable,
                        callback: gio::GAsyncReadyCallback,
                        user_data: gpointer,
                    );
                    pub fn gcr_certificate_request_capable_finish(
                        result: *mut gio::GAsyncResult,
                        error: *mut *mut glib_ffi::GError,
                    ) -> gboolean;
                    pub fn gcr_certificate_request_prepare(
                        format: GcrCertificateRequestFormat,
                        private_key: *mut gck::GckObject,
                    ) -> *mut GcrCertificateRequest;
                    pub fn gcr_certificate_request_complete(
                        self_: *mut GcrCertificateRequest,
                        cancellable: *mut gio::GCancellable,
                        error: *mut *mut glib_ffi::GError,
                    ) -> gboolean;
                    pub fn gcr_certificate_request_complete_async(
                        self_: *mut GcrCertificateRequest,
                        cancellable: *mut gio::GCancellable,
                        callback: gio::GAsyncReadyCallback,
                        user_data: gpointer,
                    );
                    pub fn gcr_certificate_request_complete_finish(
                        self_: *mut GcrCertificateRequest,
                        result: *mut gio::GAsyncResult,
                        error: *mut *mut glib_ffi::GError,
                    ) -> gboolean;
                    pub fn gcr_certificate_request_encode(
                        self_: *mut GcrCertificateRequest,
                        textual: gboolean,
                        length: *mut size_t,
                    ) -> *mut u8;
                    pub fn gcr_certificate_request_get_format(
                        self_: *mut GcrCertificateRequest,
                    ) -> GcrCertificateRequestFormat;
                    pub fn gcr_certificate_request_get_private_key(
                        self_: *mut GcrCertificateRequest,
                    ) -> *mut gck::GckObject;
                    pub fn gcr_certificate_request_set_cn(self_: *mut GcrCertificateRequest, cn: *const c_char);
    */
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
    /*
                //=========================================================================
                // GcrParser
                //=========================================================================
                pub fn gcr_parser_get_type() -> GType;
                pub fn gcr_parser_new() -> *mut GcrParser;
                pub fn gcr_parser_add_password(self_: *mut GcrParser, password: *const c_char);
                pub fn gcr_parser_format_disable(self_: *mut GcrParser, format: GcrDataFormat);
                pub fn gcr_parser_format_enable(self_: *mut GcrParser, format: GcrDataFormat);
                pub fn gcr_parser_format_supported(self_: *mut GcrParser, format: GcrDataFormat) -> gboolean;
                pub fn gcr_parser_get_filename(self_: *mut GcrParser) -> *const c_char;
                pub fn gcr_parser_get_parsed(self_: *mut GcrParser) -> *mut GcrParsed;
                pub fn gcr_parser_get_parsed_attributes(self_: *mut GcrParser) -> *mut gck::GckAttributes;
                pub fn gcr_parser_get_parsed_block(self_: *mut GcrParser, n_block: *mut size_t) -> *const u8;
                pub fn gcr_parser_get_parsed_bytes(self_: *mut GcrParser) -> *mut glib_ffi::GBytes;
                pub fn gcr_parser_get_parsed_description(self_: *mut GcrParser) -> *const c_char;
                pub fn gcr_parser_get_parsed_format(self_: *mut GcrParser) -> GcrDataFormat;
                pub fn gcr_parser_get_parsed_label(self_: *mut GcrParser) -> *const c_char;
                pub fn gcr_parser_parse_bytes(
                    self_: *mut GcrParser,
                    data: *mut glib_ffi::GBytes,
                    error: *mut *mut glib_ffi::GError,
                ) -> gboolean;
                pub fn gcr_parser_parse_data(
                    self_: *mut GcrParser,
                    data: *const u8,
                    n_data: size_t,
                    error: *mut *mut glib_ffi::GError,
                ) -> gboolean;
                pub fn gcr_parser_parse_stream(
                    self_: *mut GcrParser,
                    input: *mut gio::GInputStream,
                    cancellable: *mut gio::GCancellable,
                    error: *mut *mut glib_ffi::GError,
                ) -> gboolean;
                pub fn gcr_parser_parse_stream_async(
                    self_: *mut GcrParser,
                    input: *mut gio::GInputStream,
                    cancellable: *mut gio::GCancellable,
                    callback: gio::GAsyncReadyCallback,
                    user_data: gpointer,
                );
                pub fn gcr_parser_parse_stream_finish(
                    self_: *mut GcrParser,
                    result: *mut gio::GAsyncResult,
                    error: *mut *mut glib_ffi::GError,
                ) -> gboolean;
                pub fn gcr_parser_set_filename(self_: *mut GcrParser, filename: *const c_char);

                //=========================================================================
                // GcrPkcs11Certificate
                //=========================================================================
                pub fn gcr_pkcs11_certificate_get_type() -> GType;
                pub fn gcr_pkcs11_certificate_lookup_issuer(
                    certificate: *mut GcrCertificate,
                    cancellable: *mut gio::GCancellable,
                    error: *mut *mut glib_ffi::GError,
                ) -> *mut GcrCertificate;
                pub fn gcr_pkcs11_certificate_lookup_issuer_async(
                    certificate: *mut GcrCertificate,
                    cancellable: *mut gio::GCancellable,
                    callback: gio::GAsyncReadyCallback,
                    user_data: gpointer,
                );
                pub fn gcr_pkcs11_certificate_lookup_issuer_finish(
                    result: *mut gio::GAsyncResult,
                    error: *mut *mut glib_ffi::GError,
                ) -> *mut GcrCertificate;
                pub fn gcr_pkcs11_certificate_new_from_uri(
                    pkcs11_uri: *const c_char,
                    cancellable: *mut gio::GCancellable,
                    error: *mut *mut glib_ffi::GError,
                ) -> *mut GcrCertificate;
                pub fn gcr_pkcs11_certificate_new_from_uri_async(
                    pkcs11_uri: *const c_char,
                    cancellable: *mut gio::GCancellable,
                    callback: gio::GAsyncReadyCallback,
                    user_data: gpointer,
                );
                pub fn gcr_pkcs11_certificate_new_from_uri_finish(
                    result: *mut gio::GAsyncResult,
                    error: *mut *mut glib_ffi::GError,
                ) -> *mut GcrCertificate;
                pub fn gcr_pkcs11_certificate_get_attributes(
                    self_: *mut GcrPkcs11Certificate,
                ) -> *mut gck::GckAttributes;

                //=========================================================================
                // GcrSecretExchange
                //=========================================================================
                pub fn gcr_secret_exchange_get_type() -> GType;
                pub fn gcr_secret_exchange_new(protocol: *const c_char) -> *mut GcrSecretExchange;
                pub fn gcr_secret_exchange_begin(self_: *mut GcrSecretExchange) -> *mut c_char;
                pub fn gcr_secret_exchange_get_protocol(self_: *mut GcrSecretExchange) -> *const c_char;
                pub fn gcr_secret_exchange_get_secret(
                    self_: *mut GcrSecretExchange,
                    secret_len: *mut size_t,
                ) -> *const c_char;
                pub fn gcr_secret_exchange_receive(
                    self_: *mut GcrSecretExchange,
                    exchange: *const c_char,
                ) -> gboolean;
                pub fn gcr_secret_exchange_send(
                    self_: *mut GcrSecretExchange,
                    secret: *const c_char,
                    secret_len: ssize_t,
                ) -> *mut c_char;
    */
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
    /*
            //=========================================================================
            // GcrSshAskpass
            //=========================================================================
            pub fn gcr_ssh_askpass_get_type() -> GType;
            pub fn gcr_ssh_askpass_new(interaction: *mut gio::GTlsInteraction) -> *mut GcrSshAskpass;
            pub fn gcr_ssh_askpass_child_setup(askpass: gpointer);
            pub fn gcr_ssh_askpass_get_interaction(self_: *mut GcrSshAskpass) -> *mut gio::GTlsInteraction;

            //=========================================================================
            // GcrSystemPrompt
            //=========================================================================
            pub fn gcr_system_prompt_get_type() -> GType;
            pub fn gcr_system_prompt_error_get_domain() -> glib_ffi::GQuark;
            pub fn gcr_system_prompt_open(
                timeout_seconds: c_int,
                cancellable: *mut gio::GCancellable,
                error: *mut *mut glib_ffi::GError,
            ) -> *mut GcrSystemPrompt;
            pub fn gcr_system_prompt_open_async(
                timeout_seconds: c_int,
                cancellable: *mut gio::GCancellable,
                callback: gio::GAsyncReadyCallback,
                user_data: gpointer,
            );
            pub fn gcr_system_prompt_open_finish(
                result: *mut gio::GAsyncResult,
                error: *mut *mut glib_ffi::GError,
            ) -> *mut GcrSystemPrompt;
            pub fn gcr_system_prompt_open_for_prompter(
                prompter_name: *const c_char,
                timeout_seconds: c_int,
                cancellable: *mut gio::GCancellable,
                error: *mut *mut glib_ffi::GError,
            ) -> *mut GcrSystemPrompt;
            pub fn gcr_system_prompt_open_for_prompter_async(
                prompter_name: *const c_char,
                timeout_seconds: c_int,
                cancellable: *mut gio::GCancellable,
                callback: gio::GAsyncReadyCallback,
                user_data: gpointer,
            );
            pub fn gcr_system_prompt_close(
                self_: *mut GcrSystemPrompt,
                cancellable: *mut gio::GCancellable,
                error: *mut *mut glib_ffi::GError,
            ) -> gboolean;
            pub fn gcr_system_prompt_close_async(
                self_: *mut GcrSystemPrompt,
                cancellable: *mut gio::GCancellable,
                callback: gio::GAsyncReadyCallback,
                user_data: gpointer,
            );
            pub fn gcr_system_prompt_close_finish(
                self_: *mut GcrSystemPrompt,
                result: *mut gio::GAsyncResult,
                error: *mut *mut glib_ffi::GError,
            ) -> gboolean;
            pub fn gcr_system_prompt_get_secret_exchange(
                self_: *mut GcrSystemPrompt,
            ) -> *mut GcrSecretExchange;

            //=========================================================================
            // GcrSystemPrompter
            //=========================================================================
            pub fn gcr_system_prompter_get_type() -> GType;
            pub fn gcr_system_prompter_new(
                mode: GcrSystemPrompterMode,
                prompt_type: GType,
            ) -> *mut GcrSystemPrompter;
            pub fn gcr_system_prompter_get_mode(self_: *mut GcrSystemPrompter) -> GcrSystemPrompterMode;
            pub fn gcr_system_prompter_get_prompt_type(self_: *mut GcrSystemPrompter) -> GType;
            pub fn gcr_system_prompter_get_prompting(self_: *mut GcrSystemPrompter) -> gboolean;
            pub fn gcr_system_prompter_register(
                self_: *mut GcrSystemPrompter,
                connection: *mut gio::GDBusConnection,
            );
            pub fn gcr_system_prompter_unregister(self_: *mut GcrSystemPrompter, wait: gboolean);
    */
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
    /*
        //=========================================================================
        // GcrImportInteraction
        //=========================================================================
        pub fn gcr_import_interaction_get_type() -> GType;
        pub fn gcr_import_interaction_supplement(
            interaction: *mut GcrImportInteraction,
            builder: *mut gck::GckBuilder,
            cancellable: *mut gio::GCancellable,
            error: *mut *mut glib_ffi::GError,
        ) -> gio::GTlsInteractionResult;
        pub fn gcr_import_interaction_supplement_async(
            interaction: *mut GcrImportInteraction,
            builder: *mut gck::GckBuilder,
            cancellable: *mut gio::GCancellable,
            callback: gio::GAsyncReadyCallback,
            user_data: gpointer,
        );
        pub fn gcr_import_interaction_supplement_finish(
            interaction: *mut GcrImportInteraction,
            result: *mut gio::GAsyncResult,
            error: *mut *mut glib_ffi::GError,
        ) -> gio::GTlsInteractionResult;
        pub fn gcr_import_interaction_supplement_prep(
            interaction: *mut GcrImportInteraction,
            builder: *mut gck::GckBuilder,
        );

        //=========================================================================
        // GcrImporter
        //=========================================================================
        pub fn gcr_importer_get_type() -> GType;
        pub fn gcr_importer_create_for_parsed(parsed: *mut GcrParsed) -> *mut glib_ffi::GList;
        pub fn gcr_importer_queue_and_filter_for_parsed(
            importers: *mut glib_ffi::GList,
            parsed: *mut GcrParsed,
        ) -> *mut glib_ffi::GList;
        pub fn gcr_importer_register(importer_type: GType, attrs: *mut gck::GckAttributes);
        pub fn gcr_importer_register_well_known();
        pub fn gcr_importer_get_interaction(importer: *mut GcrImporter) -> *mut gio::GTlsInteraction;
        pub fn gcr_importer_import_async(
            importer: *mut GcrImporter,
            cancellable: *mut gio::GCancellable,
            callback: gio::GAsyncReadyCallback,
            user_data: gpointer,
        );
        pub fn gcr_importer_import_finish(
            importer: *mut GcrImporter,
            result: *mut gio::GAsyncResult,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_importer_queue_for_parsed(
            importer: *mut GcrImporter,
            parsed: *mut GcrParsed,
        ) -> gboolean;
        pub fn gcr_importer_set_interaction(
            importer: *mut GcrImporter,
            interaction: *mut gio::GTlsInteraction,
        );

        //=========================================================================
        // GcrPrompt
        //=========================================================================
        pub fn gcr_prompt_get_type() -> GType;
        pub fn gcr_prompt_close(prompt: *mut GcrPrompt);
        pub fn gcr_prompt_confirm(
            prompt: *mut GcrPrompt,
            cancellable: *mut gio::GCancellable,
            error: *mut *mut glib_ffi::GError,
        ) -> GcrPromptReply;
        pub fn gcr_prompt_confirm_async(
            prompt: *mut GcrPrompt,
            cancellable: *mut gio::GCancellable,
            callback: gio::GAsyncReadyCallback,
            user_data: gpointer,
        );
        pub fn gcr_prompt_confirm_finish(
            prompt: *mut GcrPrompt,
            result: *mut gio::GAsyncResult,
            error: *mut *mut glib_ffi::GError,
        ) -> GcrPromptReply;
        pub fn gcr_prompt_confirm_run(
            prompt: *mut GcrPrompt,
            cancellable: *mut gio::GCancellable,
            error: *mut *mut glib_ffi::GError,
        ) -> GcrPromptReply;
        pub fn gcr_prompt_get_caller_window(prompt: *mut GcrPrompt) -> *mut c_char;
        pub fn gcr_prompt_get_cancel_label(prompt: *mut GcrPrompt) -> *mut c_char;
        pub fn gcr_prompt_get_choice_chosen(prompt: *mut GcrPrompt) -> gboolean;
        pub fn gcr_prompt_get_choice_label(prompt: *mut GcrPrompt) -> *mut c_char;
        pub fn gcr_prompt_get_continue_label(prompt: *mut GcrPrompt) -> *mut c_char;
        pub fn gcr_prompt_get_description(prompt: *mut GcrPrompt) -> *mut c_char;
        pub fn gcr_prompt_get_message(prompt: *mut GcrPrompt) -> *mut c_char;
        pub fn gcr_prompt_get_password_new(prompt: *mut GcrPrompt) -> gboolean;
        pub fn gcr_prompt_get_password_strength(prompt: *mut GcrPrompt) -> c_int;
        pub fn gcr_prompt_get_title(prompt: *mut GcrPrompt) -> *mut c_char;
        pub fn gcr_prompt_get_warning(prompt: *mut GcrPrompt) -> *mut c_char;
        pub fn gcr_prompt_password(
            prompt: *mut GcrPrompt,
            cancellable: *mut gio::GCancellable,
            error: *mut *mut glib_ffi::GError,
        ) -> *const c_char;
        pub fn gcr_prompt_password_async(
            prompt: *mut GcrPrompt,
            cancellable: *mut gio::GCancellable,
            callback: gio::GAsyncReadyCallback,
            user_data: gpointer,
        );
        pub fn gcr_prompt_password_finish(
            prompt: *mut GcrPrompt,
            result: *mut gio::GAsyncResult,
            error: *mut *mut glib_ffi::GError,
        ) -> *const c_char;
        pub fn gcr_prompt_password_run(
            prompt: *mut GcrPrompt,
            cancellable: *mut gio::GCancellable,
            error: *mut *mut glib_ffi::GError,
        ) -> *const c_char;
        pub fn gcr_prompt_reset(prompt: *mut GcrPrompt);
        pub fn gcr_prompt_set_caller_window(prompt: *mut GcrPrompt, window_id: *const c_char);
        pub fn gcr_prompt_set_cancel_label(prompt: *mut GcrPrompt, cancel_label: *const c_char);
        pub fn gcr_prompt_set_choice_chosen(prompt: *mut GcrPrompt, chosen: gboolean);
        pub fn gcr_prompt_set_choice_label(prompt: *mut GcrPrompt, choice_label: *const c_char);
        pub fn gcr_prompt_set_continue_label(prompt: *mut GcrPrompt, continue_label: *const c_char);
        pub fn gcr_prompt_set_description(prompt: *mut GcrPrompt, description: *const c_char);
        pub fn gcr_prompt_set_message(prompt: *mut GcrPrompt, message: *const c_char);
        pub fn gcr_prompt_set_password_new(prompt: *mut GcrPrompt, new_password: gboolean);
        pub fn gcr_prompt_set_title(prompt: *mut GcrPrompt, title: *const c_char);
        pub fn gcr_prompt_set_warning(prompt: *mut GcrPrompt, warning: *const c_char);

        //=========================================================================
        // Other functions
        //=========================================================================
        pub fn gcr_data_error_get_domain() -> glib_ffi::GQuark;
        pub fn gcr_fingerprint_from_attributes(
            attrs: *mut gck::GckAttributes,
            checksum_type: glib_ffi::GChecksumType,
            n_fingerprint: *mut size_t,
        ) -> *mut u8;
        pub fn gcr_fingerprint_from_subject_public_key_info(
            key_info: *const u8,
            n_key_info: size_t,
            checksum_type: glib_ffi::GChecksumType,
            n_fingerprint: *mut size_t,
        ) -> *mut u8;
        pub fn gcr_mock_prompter_disconnect();
        pub fn gcr_mock_prompter_expect_close();
        pub fn gcr_mock_prompter_expect_confirm_cancel();
        pub fn gcr_mock_prompter_expect_confirm_ok(first_property_name: *const c_char, ...);
        pub fn gcr_mock_prompter_expect_password_cancel();
        pub fn gcr_mock_prompter_expect_password_ok(
            password: *const c_char,
            first_property_name: *const c_char,
            ...
        );
        pub fn gcr_mock_prompter_get_delay_msec() -> c_uint;
        pub fn gcr_mock_prompter_is_expecting() -> gboolean;
        pub fn gcr_mock_prompter_is_prompting() -> gboolean;
        pub fn gcr_mock_prompter_set_delay_msec(delay_msec: c_uint);
        pub fn gcr_mock_prompter_start() -> *const c_char;
        pub fn gcr_mock_prompter_stop();
        pub fn gcr_pkcs11_add_module(module: *mut gck::GckModule);
        pub fn gcr_pkcs11_add_module_from_file(
            module_path: *const c_char,
            unused: gpointer,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_pkcs11_get_modules() -> *mut glib_ffi::GList;
        pub fn gcr_pkcs11_get_trust_lookup_slots() -> *mut glib_ffi::GList;
        pub fn gcr_pkcs11_get_trust_lookup_uris() -> *mut *const c_char;
        pub fn gcr_pkcs11_get_trust_store_slot() -> *mut gck::GckSlot;
        pub fn gcr_pkcs11_get_trust_store_uri() -> *const c_char;
        pub fn gcr_pkcs11_initialize(
            cancellable: *mut gio::GCancellable,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_pkcs11_initialize_async(
            cancellable: *mut gio::GCancellable,
            callback: gio::GAsyncReadyCallback,
            user_data: gpointer,
        );
        pub fn gcr_pkcs11_initialize_finish(
            result: *mut gio::GAsyncResult,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_pkcs11_set_modules(modules: *mut glib_ffi::GList);
        pub fn gcr_pkcs11_set_trust_lookup_uris(pkcs11_uris: *mut *const c_char);
        pub fn gcr_pkcs11_set_trust_store_uri(pkcs11_uri: *const c_char);
        pub fn gcr_secure_memory_alloc(size: size_t) -> gpointer;
        pub fn gcr_secure_memory_free(memory: gpointer);
        pub fn gcr_secure_memory_is_secure(memory: gpointer) -> gboolean;
        pub fn gcr_secure_memory_realloc(memory: gpointer, size: size_t) -> gpointer;
        pub fn gcr_secure_memory_strdup(string: *const c_char) -> *mut c_char;
        pub fn gcr_secure_memory_strfree(string: *mut c_char);
        pub fn gcr_secure_memory_try_alloc(size: size_t) -> gpointer;
        pub fn gcr_secure_memory_try_realloc(memory: gpointer, size: size_t) -> gpointer;
        pub fn gcr_trust_add_pinned_certificate(
            certificate: *mut GcrCertificate,
            purpose: *const c_char,
            peer: *const c_char,
            cancellable: *mut gio::GCancellable,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_trust_add_pinned_certificate_async(
            certificate: *mut GcrCertificate,
            purpose: *const c_char,
            peer: *const c_char,
            cancellable: *mut gio::GCancellable,
            callback: gio::GAsyncReadyCallback,
            user_data: gpointer,
        );
        pub fn gcr_trust_add_pinned_certificate_finish(
            result: *mut gio::GAsyncResult,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_trust_is_certificate_anchored(
            certificate: *mut GcrCertificate,
            purpose: *const c_char,
            cancellable: *mut gio::GCancellable,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_trust_is_certificate_anchored_async(
            certificate: *mut GcrCertificate,
            purpose: *const c_char,
            cancellable: *mut gio::GCancellable,
            callback: gio::GAsyncReadyCallback,
            user_data: gpointer,
        );
        pub fn gcr_trust_is_certificate_anchored_finish(
            result: *mut gio::GAsyncResult,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_trust_is_certificate_distrusted(
            serial_nr: *mut u8,
            serial_nr_len: size_t,
            issuer: *mut u8,
            issuer_len: size_t,
            cancellable: *mut gio::GCancellable,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_trust_is_certificate_distrusted_async(
            serial_nr: *mut u8,
            serial_nr_len: size_t,
            issuer: *mut u8,
            issuer_len: size_t,
            cancellable: *mut gio::GCancellable,
            callback: gio::GAsyncReadyCallback,
            user_data: *mut c_void,
        );
        pub fn gcr_trust_is_certificate_distrusted_finish(
            result: *mut gio::GAsyncResult,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_trust_is_certificate_pinned(
            certificate: *mut GcrCertificate,
            purpose: *const c_char,
            peer: *const c_char,
            cancellable: *mut gio::GCancellable,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_trust_is_certificate_pinned_async(
            certificate: *mut GcrCertificate,
            purpose: *const c_char,
            peer: *const c_char,
            cancellable: *mut gio::GCancellable,
            callback: gio::GAsyncReadyCallback,
            user_data: gpointer,
        );
        pub fn gcr_trust_is_certificate_pinned_finish(
            result: *mut gio::GAsyncResult,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_trust_remove_pinned_certificate(
            certificate: *mut GcrCertificate,
            purpose: *const c_char,
            peer: *const c_char,
            cancellable: *mut gio::GCancellable,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
        pub fn gcr_trust_remove_pinned_certificate_async(
            certificate: *mut GcrCertificate,
            purpose: *const c_char,
            peer: *const c_char,
            cancellable: *mut gio::GCancellable,
            callback: gio::GAsyncReadyCallback,
            user_data: gpointer,
        );
        pub fn gcr_trust_remove_pinned_certificate_finish(
            result: *mut gio::GAsyncResult,
            error: *mut *mut glib_ffi::GError,
        ) -> gboolean;
    */
}
