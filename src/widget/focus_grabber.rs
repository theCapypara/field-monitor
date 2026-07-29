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

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::WeakRef;
use gtk::glib;
use gtk::{BinLayout, gdk};
use log::{debug, trace};
use rdw::DisplayExt;
use std::cell::Cell;
use std::cell::RefCell;

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
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<BinLayout>();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorFocusGrabber {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // -- Forwarding input

            // We send mouse motion to the RDW display always. Clicks are delivered
            // exactly from when the input is grabbed (including the click to grab itself).
            let controller = gtk::GestureClick::builder()
                .propagation_phase(gtk::PropagationPhase::Bubble)
                .button(0)
                .build();
            controller.connect_released(glib::clone!(
                #[weak]
                obj,
                move |_, _, _, _| obj.grab()
            ));
            obj.add_controller(controller);

            // Scrolling however should not reach the display while not grabbed
            let scroll_controller = gtk::EventControllerScroll::builder()
                .propagation_phase(gtk::PropagationPhase::Capture)
                .flags(gtk::EventControllerScrollFlags::BOTH_AXES)
                .build();
            scroll_controller.connect_scroll(glib::clone!(
                #[weak]
                obj,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_, _, _| {
                    if obj.grabbed() {
                        glib::Propagation::Proceed
                    } else {
                        glib::Propagation::Stop
                    }
                }
            ));
            obj.add_controller(scroll_controller);

            // For key presses, we filter some out when ungrabbed so tab element navigation
            // works (a11y) and special keys (especially Super) aren't forwarded to the
            // display unexpectedly. All other key presses cause a grab and send that key
            // to the RDW display immediately.
            let key_controller = gtk::EventControllerKey::builder()
                .propagation_phase(gtk::PropagationPhase::Bubble)
                .build();
            key_controller.connect_key_pressed(glib::clone!(
                #[weak(rename_to=slf)]
                self,
                #[weak]
                obj,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_, key, keycode, state| {
                    if !slf.grabbed.get() {
                        if key == gdk::Key::Shift_L
                            || key == gdk::Key::Shift_R
                            || key == gdk::Key::Control_L
                            || key == gdk::Key::Control_R
                            || key == gdk::Key::Alt_L
                            || key == gdk::Key::Alt_R
                            || key == gdk::Key::Super_L
                            || key == gdk::Key::Super_R
                        {
                            glib::Propagation::Stop
                        } else if key == gdk::Key::Tab || key == gdk::Key::ISO_Left_Tab {
                            debug!("tab pressed in focus grabber");
                            obj.emit_move_focus(if state.contains(gdk::ModifierType::SHIFT_MASK) {
                                gtk::DirectionType::TabBackward
                            } else {
                                gtk::DirectionType::TabForward
                            });
                            glib::Propagation::Stop
                        } else {
                            debug!("focused grabbed via keyboard in focus grabber: {:?}", key);
                            obj.grab();
                            // We now need to forward this to rdw as well
                            obj.forward_key_press(key, keycode);
                            glib::Propagation::Stop
                        }
                    } else {
                        glib::Propagation::Proceed
                    }
                }
            ));
            obj.add_controller(key_controller);

            self.obj().set_focusable(true);
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
            // When we are not grabbed the rdw display should not be focusable, but still sensitive
            // so it properly handles things like motion.
            if let Some(child) = self.obj().first_child() {
                self.obj()
                    .bind_property("grabbed", &child, "can-focus")
                    .sync_create()
                    .build();
            }
        }
    }

    impl Drop for FieldMonitorFocusGrabber {
        fn drop(&mut self) {
            debug!("drop FieldMonitorFocusGrabber");
            if let Some(display) = self.display.borrow().as_ref().and_then(WeakRef::upgrade)
                && let Some(display_signal_id) = self.display_signal_id.borrow_mut().take()
            {
                display.disconnect(display_signal_id);
            }
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorFocusGrabber(ObjectSubclass<imp::FieldMonitorFocusGrabber>)
        @extends gtk::Widget,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
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
            imp.display_signal_id
                .replace(Some(display.connect_property_grabbed_notify(glib::clone!(
                    #[weak(rename_to = slf)]
                    self,
                    move |_| {
                        slf.on_inner_grab_changed();
                    }
                ))));
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

        if let Some(display) = self.display() {
            let grab = display.try_grab();
            display.grab_focus();
            debug!("try_grab result: {grab:?}");
        }
        self.try_mute_accels(true);
    }

    pub fn ungrab(&self) {
        let imp = self.imp();
        if !imp.grabbed.get() {
            return;
        }

        imp.grabbed.set(false);
        self.notify_grabbed();

        if let Some(display) = self.display() {
            display.ungrab();

            if let Some(root) = self.root()
                && root.focus().is_none()
            {
                self.grab_focus();
                debug!("self grab focus");
            } else if recursive_check_has_focus(display.upcast_ref()) {
                self.grab_focus();
                debug!("self grab focus");
            }
            debug!("ungrab");
            Self::release_all_held_mod_keys(&display);
        }
        self.try_mute_accels(false);
    }

    fn try_mute_accels(&self, mute: bool) {
        if let Some(fm_app) = self
            .root()
            .and_downcast::<adw::ApplicationWindow>()
            .and_then(|win| win.application())
            .and_downcast::<FieldMonitorApplication>()
        {
            if mute {
                fm_app.remove_accels();
            } else {
                fm_app.add_accels();
            }
        }
    }

    fn on_inner_grab_changed(&self) {
        if let Some(inner_grabbed) = self.display().as_ref().map(DisplayExt::grabbed) {
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

    fn display(&self) -> Option<rdw::Display> {
        self.imp()
            .display
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
    }

    fn forward_key_press(&self, key: gdk::Key, keycode: u32) {
        if let Some(display) = self.display()
            && !display.read_only()
        {
            trace!("forwarding grabbing key press: {key:?}");
            display.emit_by_name::<()>("key-event", &[&key, &keycode, &rdw::KeyEvent::PRESS]);
        }
    }

    /// Release CTRL, ALT and TAB on the rdw display
    fn release_all_held_mod_keys(display: &rdw::Display) {
        // a bit of a hack tbh
        glib::idle_add_local_once(glib::clone!(
            #[weak]
            display,
            move || {
                let gdk_display = display.display();
                trace!("release_all_held_mod_keys");
                for kv in [
                    gdk::Key::Alt_L,
                    gdk::Key::Alt_R,
                    gdk::Key::Control_L,
                    gdk::Key::Control_R,
                    gdk::Key::Tab,
                ] {
                    if let Some(kks) = gdk_display.map_keyval(kv) {
                        for kk in kks {
                            trace!("emit key-event release for {kk:?}");
                            display.emit_by_name::<()>(
                                "key-event",
                                &[&kv, &kk.keycode(), &rdw::KeyEvent::RELEASE],
                            );
                        }
                    }
                }
            }
        ));
    }
}

fn recursive_check_has_focus(widget: &gtk::Widget) -> bool {
    if widget.has_focus() {
        true
    } else {
        if let Some(first_child) = widget.first_child() {
            if recursive_check_has_focus(&first_child) {
                true
            } else {
                let mut current = first_child;
                while let Some(sibling) = current.next_sibling() {
                    if recursive_check_has_focus(&sibling) {
                        return true;
                    }
                    current = sibling;
                }
                false
            }
        } else {
            false
        }
    }
}
