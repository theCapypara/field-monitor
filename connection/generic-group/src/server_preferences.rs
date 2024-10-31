use std::cell::RefCell;
use std::num::NonZeroU32;

use crate::credential_preferences::GenericGroupCredentialPreferences;
use crate::preferences::{GenericGroupConfiguration, ServerType};
use crate::server_config::FinalizedServerConfig;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::GenericGroupServerPreferences)]
    #[template(
        resource = "/de/capypara/FieldMonitor/connection/generic-group/server_preferences.ui"
    )]
    pub struct GenericGroupServerPreferences {
        #[template_child]
        pub(crate) server_type_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(crate) title_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(crate) host_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(crate) port_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(crate) credentials: TemplateChild<GenericGroupCredentialPreferences>,

        #[property(get, construct_only)]
        pub key: RefCell<String>,
        #[property(get, set)]
        pub server_type: RefCell<String>,
        #[property(get, set)]
        pub title: RefCell<String>,
        #[property(get, set)]
        pub host: RefCell<String>,
        #[property(get, set)]
        pub port: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GenericGroupServerPreferences {
        const NAME: &'static str = "GenericGroupServerPreferences";
        type Type = super::GenericGroupServerPreferences;
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
    impl ObjectImpl for GenericGroupServerPreferences {}
    impl WidgetImpl for GenericGroupServerPreferences {}
    impl PreferencesPageImpl for GenericGroupServerPreferences {}
}

glib::wrapper! {
    pub struct GenericGroupServerPreferences(ObjectSubclass<imp::GenericGroupServerPreferences>)
        @extends gtk::Widget, adw::PreferencesPage;
}

impl GenericGroupServerPreferences {
    pub fn new<T>(server: &str, existing_configuration: Option<T>) -> Self
    where
        T: GenericGroupConfiguration + 'static,
    {
        let slf: Self = glib::Object::builder().property("key", server).build();

        let server = server.to_string();
        if let Some(existing_configuration) = existing_configuration {
            glib::spawn_future_local(glib::clone!(
                #[weak]
                slf,
                async move {
                    if let Some(v) = existing_configuration.server_type(&server) {
                        slf.set_server_type(v.to_string());
                    }
                    if let Some(v) = existing_configuration.title(&server) {
                        slf.set_title(v);
                    }
                    if let Some(v) = existing_configuration.host(&server) {
                        slf.set_host(v);
                    }
                    if let Some(v) = existing_configuration.port(&server) {
                        slf.set_port(v.to_string());
                    }

                    slf.imp()
                        .credentials
                        .propagate_settings(&server, &existing_configuration)
                        .await;
                }
            ));
        } else {
            slf.set_server_type(ServerType::Rdp.to_string());
        }
        slf
    }
    pub fn credentials(&self) -> &GenericGroupCredentialPreferences {
        &self.imp().credentials
    }

    pub fn make_config(&self) -> Option<FinalizedServerConfig> {
        let mut config = FinalizedServerConfig::default();
        let Some(port) = self
            .port()
            .parse::<u32>()
            .ok()
            .and_then(|v| NonZeroU32::try_from(v).ok())
        else {
            self.port_entry_error(true);
            return None;
        };
        self.port_entry_error(false);

        config.title = self.title();
        debug_assert!(ServerType::try_from(self.server_type()).is_ok());
        config.server_type = self.server_type().try_into().ok();
        config.host = self.host();
        config.port = port;
        config.key = self.key();
        self.imp().credentials.update_server_config(&mut config);
        Some(config)
    }

    pub fn port_entry_error(&self, error: bool) {
        if error {
            self.imp().port_entry.add_css_class("error");
        } else {
            self.imp().port_entry.remove_css_class("error");
        }
    }
}

#[gtk::template_callbacks]
impl GenericGroupServerPreferences {
    const SELECTED_IDX_RDP: u32 = 0;
    const SELECTED_IDX_SPICE: u32 = 1;
    const SELECTED_IDX_VNC: u32 = 2;

    #[template_callback]
    fn on_self_server_type_changed(&self) {
        let server_type: Option<ServerType> = self.server_type().try_into().ok();
        self.imp().server_type_row.set_selected(match server_type {
            Some(ServerType::Rdp) => Self::SELECTED_IDX_RDP,
            Some(ServerType::Spice) => Self::SELECTED_IDX_SPICE,
            Some(ServerType::Vnc) => Self::SELECTED_IDX_VNC,
            _ => return,
        });
    }

    #[template_callback]
    fn on_server_type_combo_selected(&self) {
        let server_type = match self.imp().server_type_row.selected() {
            Self::SELECTED_IDX_RDP => ServerType::Rdp,
            Self::SELECTED_IDX_SPICE => ServerType::Spice,
            Self::SELECTED_IDX_VNC => ServerType::Vnc,
            _ => return,
        };
        self.set_server_type(server_type.to_string());
    }
}
