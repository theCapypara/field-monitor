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

use crate::application::FieldMonitorApplication;
use crate::settings::{SettingHeaderBarBehavior, SettingSharpWindowCorners};
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorPreferencesDialog)]
    #[template(resource = "/de/capypara/FieldMonitor/widget/preferences.ui")]
    pub struct FieldMonitorPreferencesDialog {
        #[template_child]
        pub sharp_window_corners_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub header_bar_behavior_label: TemplateChild<gtk::Label>,

        #[property(get, construct_only)]
        pub application: RefCell<Option<FieldMonitorApplication>>,
        #[property(get, set)]
        pub sharp_window_corners: RefCell<SettingSharpWindowCorners>,
        #[property(get, set)]
        pub header_bar_behavior: RefCell<SettingHeaderBarBehavior>,
        #[property(get, set)]
        pub open_in_new_window: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorPreferencesDialog {
        const NAME: &'static str = "FieldMonitorPreferencesDialog";
        type Type = super::FieldMonitorPreferencesDialog;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorPreferencesDialog {}
    impl WidgetImpl for FieldMonitorPreferencesDialog {}
    impl AdwDialogImpl for FieldMonitorPreferencesDialog {}
    impl PreferencesDialogImpl for FieldMonitorPreferencesDialog {}
}

glib::wrapper! {
    pub struct FieldMonitorPreferencesDialog(ObjectSubclass<imp::FieldMonitorPreferencesDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog;
}

impl FieldMonitorPreferencesDialog {
    pub fn new(application: &FieldMonitorApplication) -> Self {
        let settings = application.settings().unwrap();

        let slf: Self = glib::Object::builder()
            .property("application", application)
            .build();

        settings
            .bind_property("sharp-window-corners", &slf, "sharp-window-corners")
            .bidirectional()
            .sync_create()
            .build();
        settings
            .bind_property("header-bar-behavior", &slf, "header-bar-behavior")
            .bidirectional()
            .sync_create()
            .build();
        settings
            .bind_property("open-in-new-window", &slf, "open-in-new-window")
            .bidirectional()
            .sync_create()
            .build();

        slf.on_self_sharp_window_corners_changed();
        slf.on_self_header_bar_behavior_changed();

        slf
    }

    fn make_radio_subpage(
        &self,
        initial_value: usize,
        title: String,
        description: Option<String>,
        options: &[(String, Option<String>)],
        apply: Rc<impl Fn(usize) + 'static>,
    ) -> adw::NavigationPage {
        let pref_group = adw::PreferencesGroup::new();

        let mut first_radio = None;
        for (i, (title, description)) in options.iter().enumerate() {
            let radio = gtk::CheckButton::new();
            match &first_radio {
                None => first_radio = Some(radio.clone()),
                Some(group) => radio.set_group(Some(group)),
            }
            if i == initial_value {
                radio.set_active(true);
            }
            radio.connect_toggled(glib::clone!(
                #[strong]
                apply,
                move |radio| {
                    if radio.is_active() {
                        apply(i);
                    }
                }
            ));

            let action_row = adw::ActionRow::builder()
                .title(title)
                .activatable_widget(&radio)
                .build();
            if let Some(description) = description {
                action_row.set_subtitle(description);
            }
            action_row.add_prefix(&radio);

            pref_group.add(&action_row);
        }

        let pref_page = adw::PreferencesPage::new();
        if let Some(description) = description {
            pref_page.set_description(&description);
        }
        pref_page.add(&pref_group);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.set_content(Some(&pref_page));

        adw::NavigationPage::new(&toolbar, &title)
    }
}

#[gtk::template_callbacks]
impl FieldMonitorPreferencesDialog {
    #[template_callback]
    pub fn on_self_sharp_window_corners_changed(&self) {
        let imp = self.imp();
        imp.sharp_window_corners_label
            .set_text(&match self.sharp_window_corners() {
                SettingSharpWindowCorners::Auto => gettext("Automatic"),
                SettingSharpWindowCorners::Always => gettext("Always"),
                SettingSharpWindowCorners::Never => gettext("Never"),
            })
    }
    #[template_callback]
    pub fn on_self_header_bar_behavior_changed(&self) {
        let imp = self.imp();
        imp.header_bar_behavior_label
            .set_text(&match self.header_bar_behavior() {
                SettingHeaderBarBehavior::Default => gettext("Default"),
                SettingHeaderBarBehavior::NoOverlay => gettext("Show Above"),
                SettingHeaderBarBehavior::Overlay => gettext("Overlay and Hide"),
            })
    }

    #[template_callback]
    pub fn on_sharp_window_corners_row_activated(&self) {
        self.push_subpage(&self.make_radio_subpage(
            match self.sharp_window_corners() {
                SettingSharpWindowCorners::Auto => 0,
                SettingSharpWindowCorners::Always => 1,
                SettingSharpWindowCorners::Never => 2
            },
            gettext("Sharp window corners"),
            Some(gettext("Configure whether and when Field Monitor will use sharp, right angle, window corners instead of the default corner radius. This is useful to make sure the corners of connected screens are not cut off.")),
            &[
                (gettext("Automatic"), Some(gettext("Use default corner radius, but make window corners sharp whenever input is grabbed."))),
                (gettext("Always"), Some(gettext("Always use sharp window corners."))),
                (gettext("Never"), Some(gettext("Never use sharp window corners."))),
            ],
            Rc::new(glib::clone!(
                #[weak(rename_to=slf)]
                self,
                move |option_idx| {
                    slf.set_sharp_window_corners(match option_idx {
                        0 => SettingSharpWindowCorners::Auto,
                        1 => SettingSharpWindowCorners::Always,
                        2 => SettingSharpWindowCorners::Never,
                        _ => unreachable!(),
                    });
                }
            ),
        )))
    }

    #[template_callback]
    pub fn on_header_bar_behavior_row_activated(&self) {
        self.push_subpage(&self.make_radio_subpage(
            match self.header_bar_behavior() {
                SettingHeaderBarBehavior::Default => 0,
                SettingHeaderBarBehavior::NoOverlay => 1,
                SettingHeaderBarBehavior::Overlay => 2,
            },
            gettext("Header bars for active connections"),
            Some(gettext("Change how the header bar is presented for active connection screens.")),
            &[
                (gettext("Default"), Some(gettext("The header bar is shown above the connection screen. In fullscreen mode the header bar is instead overlayed and hides whenever input is grabbed."))),
                (gettext("Show Above"), Some(gettext("The header bar is always shown above the connection screen, even in fullscreen mode."))),
                (gettext("Overlay and Hide"), Some(gettext("The header bar is always shown as an overlay on top of the connection screen, it is hidden whenever input is grabbed."))),
            ],
            Rc::new(glib::clone!(
                #[weak(rename_to=slf)]
                self,
                move |option_idx| {
                    slf.set_header_bar_behavior(match option_idx {
                        0 => SettingHeaderBarBehavior::Default,
                        1 => SettingHeaderBarBehavior::NoOverlay,
                        2 => SettingHeaderBarBehavior::Overlay,
                        _ => unreachable!(),
                    });
                }
            ),
        )))
    }
}
