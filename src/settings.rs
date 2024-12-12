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
use adw::gio;
use adw::prelude::*;
use glib::subclass::prelude::*;
use libfieldmonitor::impl_enum_param_spec;
use std::cell::Cell;
use std::cell::RefCell;

#[derive(Copy, Clone, Debug, Default)]
pub enum SettingSharpWindowCorners {
    #[default]
    Auto,
    Always,
    Never,
}

impl From<String> for SettingSharpWindowCorners {
    fn from(value: String) -> Self {
        match &*value {
            "always" => SettingSharpWindowCorners::Always,
            "never" => SettingSharpWindowCorners::Never,
            _ => SettingSharpWindowCorners::Auto,
        }
    }
}

impl<'a> From<&'a SettingSharpWindowCorners> for String {
    fn from(value: &'a SettingSharpWindowCorners) -> Self {
        match value {
            SettingSharpWindowCorners::Auto => "auto",
            SettingSharpWindowCorners::Always => "always",
            SettingSharpWindowCorners::Never => "never",
        }
        .to_string()
    }
}

impl_enum_param_spec!(SettingSharpWindowCorners, String);

#[derive(Copy, Clone, Debug, Default)]
pub enum SettingHeaderBarBehavior {
    #[default]
    Default,
    Overlay,
    NoOverlay,
}

impl From<String> for SettingHeaderBarBehavior {
    fn from(value: String) -> Self {
        match &*value {
            "overlay" => SettingHeaderBarBehavior::Overlay,
            "no-overlay" => SettingHeaderBarBehavior::NoOverlay,
            _ => SettingHeaderBarBehavior::Default,
        }
    }
}

impl<'a> From<&'a SettingHeaderBarBehavior> for String {
    fn from(value: &'a SettingHeaderBarBehavior) -> Self {
        match value {
            SettingHeaderBarBehavior::Default => "default",
            SettingHeaderBarBehavior::Overlay => "overlay",
            SettingHeaderBarBehavior::NoOverlay => "no-overlay",
        }
        .to_string()
    }
}

impl_enum_param_spec!(SettingHeaderBarBehavior, String);

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorSettings)]
    pub struct FieldMonitorSettings {
        #[property(get, construct_only)]
        pub settings: RefCell<Option<gio::Settings>>,

        #[property(get, set)]
        pub sharp_window_corners: RefCell<SettingSharpWindowCorners>,
        #[property(get, set)]
        pub header_bar_behavior: RefCell<SettingHeaderBarBehavior>,
        #[property(get, set)]
        pub open_in_new_window: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorSettings {
        const NAME: &'static str = "FieldMonitorSettings";
        type Type = super::FieldMonitorSettings;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorSettings {}
}

glib::wrapper! {
    pub struct FieldMonitorSettings(ObjectSubclass<imp::FieldMonitorSettings>);
}

impl FieldMonitorSettings {
    pub fn new(app_id: &str) -> Self {
        let settings = gio::Settings::new(app_id);
        let slf = glib::Object::builder()
            .property("settings", &settings)
            .build();

        settings
            .bind("sharp-window-corners", &slf, "sharp-window-corners")
            .build();
        settings
            .bind("header-bar-behavior", &slf, "header-bar-behavior")
            .build();
        settings
            .bind("open-in-new-window", &slf, "open-in-new-window")
            .build();

        slf
    }
}
