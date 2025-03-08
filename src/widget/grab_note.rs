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

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use crate::application::FieldMonitorApplication;
use crate::settings::FieldMonitorSettings;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct FieldMonitorGrabNote {
        pub timeout: Mutex<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorGrabNote {
        const NAME: &'static str = "FieldMonitorGrabNote";
        type Type = super::FieldMonitorGrabNote;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for FieldMonitorGrabNote {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.set_can_focus(false);
            obj.set_can_target(false);
            obj.add_css_class("grab-note");
            obj.add_css_class("hidden");
            obj.set_child(Some(&gtk::Label::new(None)));
        }
    }
    impl WidgetImpl for FieldMonitorGrabNote {}
    impl BinImpl for FieldMonitorGrabNote {}
}

glib::wrapper! {
    pub struct FieldMonitorGrabNote(ObjectSubclass<imp::FieldMonitorGrabNote>)
        @extends gtk::Widget, adw::Bin;
}

impl FieldMonitorGrabNote {
    pub fn show_note(&self, label: &str) {
        let show_grab_note = self
            .root()
            .and_downcast::<gtk::Window>()
            .as_ref()
            .and_then(gtk::Window::application)
            .and_downcast::<FieldMonitorApplication>()
            .as_ref()
            .and_then(FieldMonitorApplication::settings)
            .as_ref()
            .map(FieldMonitorSettings::show_grab_note)
            .unwrap_or(true);

        if !show_grab_note {
            return;
        }

        let mut timeout = self
            .imp()
            .timeout
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(timeout) = timeout.take() {
            timeout.remove();
        }

        self.remove_css_class("hidden");
        self.child()
            .and_downcast::<gtk::Label>()
            .unwrap()
            .set_label(label);

        *timeout = Some(glib::timeout_add_local(
            Duration::from_secs(5),
            glib::clone!(
                #[weak_allow_none(rename_to=slf)]
                self,
                move || {
                    if let Some(slf) = slf {
                        let mut lock = slf
                            .imp()
                            .timeout
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner);

                        slf.hide_note();

                        *lock = None;
                    }
                    glib::ControlFlow::Break
                }
            ),
        ))
    }

    pub fn hide_note(&self) {
        self.add_css_class("hidden");
    }
}
