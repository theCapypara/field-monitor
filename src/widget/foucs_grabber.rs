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

use std::cell::Cell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::WeakRef;
use gtk::glib;
use log::debug;
use rdw::DisplayExt;

use crate::application::FieldMonitorApplication;

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorFocusGrabber)]
    pub struct FieldMonitorFocusGrabber {
        #[property(get)]
        pub grabbed: Cell<bool>,
        pub display: RefCell<Option<WeakRef<rdw::Display>>>,
        pub display_signal_id: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorFocusGrabber {
        const NAME: &'static str = "FieldMonitorFocusGrabber";
        type Type = super::FieldMonitorFocusGrabber;
        type ParentType = adw::Bin;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorFocusGrabber {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_visible(false);

            let controller = gtk::GestureClick::builder()
                .propagation_phase(gtk::PropagationPhase::Capture)
                .build();

            controller.connect_released(glib::clone!(
                #[weak]
                obj,
                move |_, _, _, _| obj.grab()
            ));

            obj.add_controller(controller);
            obj.add_controller(
                gtk::EventControllerScroll::builder()
                    .propagation_phase(gtk::PropagationPhase::Capture)
                    .build(),
            );
            obj.add_controller(
                gtk::EventControllerMotion::builder()
                    .propagation_phase(gtk::PropagationPhase::Capture)
                    .build(),
            );
        }
    }
    impl WidgetImpl for FieldMonitorFocusGrabber {
        fn realize(&self) {
            self.parent_realize();

            let root = self.obj().root();
            if let Some(root) = root.map(Cast::downcast::<gtk::Window>).and_then(Result::ok) {
                root.connect_is_active_notify(glib::clone!(
                    #[weak(rename_to=slf)]
                    self,
                    move |window| slf.obj().on_window_active(window.is_active())
                ));
            }
        }
    }
    impl BinImpl for FieldMonitorFocusGrabber {}

    impl Drop for FieldMonitorFocusGrabber {
        fn drop(&mut self) {
            debug!("drop FieldMonitorFocusGrabber");
            if let Some(display) = self.display.borrow().as_ref().and_then(WeakRef::upgrade) {
                if let Some(display_signal_id) = self.display_signal_id.borrow_mut().take() {
                    display.disconnect(display_signal_id);
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorFocusGrabber(ObjectSubclass<imp::FieldMonitorFocusGrabber>)
        @extends gtk::Widget, adw::Bin;
}

impl FieldMonitorFocusGrabber {
    pub fn set_display(&self, value: Option<&rdw::Display>) {
        let imp = self.imp();
        let mut display_opt = imp.display.borrow_mut();
        if let Some(display) = display_opt.as_ref().and_then(WeakRef::upgrade) {
            let signal_id_opt = imp.display_signal_id.take();
            if let Some(display_signal_id) = signal_id_opt {
                display.disconnect(display_signal_id);
            }
        }
        if let Some(display) = &value {
            self.set_visible(true);
            imp.display_signal_id
                .replace(Some(display.connect_property_grabbed_notify(glib::clone!(
                    #[weak(rename_to = slf)]
                    self,
                    move |_| {
                        slf.on_inner_grab_changed();
                    }
                ))));
        } else {
            self.set_visible(false);
        }
        *display_opt = value.map(ObjectExt::downgrade);
    }

    fn grab(&self) {
        let imp = self.imp();
        if imp.grabbed.get() {
            return;
        }

        imp.grabbed.set(true);
        self.notify_grabbed();

        if let Some(display) = self
            .imp()
            .display
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
        {
            let grab = display.try_grab();
            debug!("try_grab result: {grab:?}");
        }
        self.try_mute_accels(true);

        self.set_visible(false);
    }

    fn ungrab(&self) {
        let imp = self.imp();
        if !imp.grabbed.get() {
            return;
        }

        imp.grabbed.set(false);
        self.notify_grabbed();

        if let Some(display) = self
            .imp()
            .display
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
        {
            display.ungrab();
            debug!("ungrab");
        }
        self.try_mute_accels(false);

        self.set_visible(true);
    }

    fn try_mute_accels(&self, mute: bool) {
        if let Some(fm_app) = self
            .root()
            .and_then(|root| root.downcast::<adw::ApplicationWindow>().ok())
            .and_then(|win| win.application())
            .and_then(|app| app.downcast::<FieldMonitorApplication>().ok())
        {
            if mute {
                fm_app.remove_accels();
            } else {
                fm_app.add_accels();
            }
        }
    }

    fn on_inner_grab_changed(&self) {
        if let Some(inner_grabbed) = self
            .imp()
            .display
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
            .map(|d| d.grabbed())
        {
            if inner_grabbed.is_empty() {
                self.ungrab();
            } else {
                self.grab();
            }
        }
    }

    fn on_window_active(&self, is_active: bool) {
        if self.grabbed() && !is_active {
            self.ungrab();
        }
    }
}
