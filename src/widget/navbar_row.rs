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
use crate::util::MOUSE_RIGHT_BUTTON;
use adw::{gdk, gio};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::FieldMonitorNavbarRow)]
    pub struct FieldMonitorNavbarRow {
        #[property(get, set)]
        pub child_ref: RefCell<Option<glib::Object>>,
        #[property(get, set = Self::set_content)]
        pub content: RefCell<Option<gtk::Widget>>,
        pub slot_left: RefCell<Option<gtk::Box>>,
        pub slot_middle: RefCell<Option<gtk::Box>>,
        pub slot_right: RefCell<Option<gtk::Box>>,
        pub action_group: RefCell<Option<gio::SimpleActionGroup>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorNavbarRow {
        const NAME: &'static str = "FieldMonitorNavbarRow";
        type Type = super::FieldMonitorNavbarRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.add_binding_action(gdk::Key::F10, gdk::ModifierType::SHIFT_MASK, "menu.popup");
            klass.add_binding_action(gdk::Key::Menu, gdk::ModifierType::empty(), "menu.popup");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FieldMonitorNavbarRow {
        fn constructed(&self) {
            let boxx = gtk::Box::builder()
                .spacing(6)
                .orientation(gtk::Orientation::Horizontal)
                .halign(gtk::Align::Fill)
                .build();

            let slot_left = gtk::Box::builder()
                .spacing(6)
                .orientation(gtk::Orientation::Horizontal)
                .halign(gtk::Align::Start)
                .build();
            let slot_middle = gtk::Box::builder()
                .spacing(6)
                .orientation(gtk::Orientation::Horizontal)
                .halign(gtk::Align::Fill)
                .build();
            let slot_right = gtk::Box::builder()
                .spacing(6)
                .orientation(gtk::Orientation::Horizontal)
                .halign(gtk::Align::End)
                .build();

            boxx.append(&slot_left);
            boxx.append(&slot_middle);
            boxx.append(&slot_right);

            self.slot_left.replace(Some(slot_left));
            self.slot_middle.replace(Some(slot_middle));
            self.slot_right.replace(Some(slot_right));

            self.obj().set_child(Some(&boxx))
        }

        fn dispose(&self) {
            self.unset_content();
            for slot in [&self.slot_left, &self.slot_right] {
                let slt_brw = slot.borrow();
                let slt_ref = slt_brw.as_ref().unwrap();
                while let Some(child) = slt_ref.last_child() {
                    child.unparent();
                }
            }
        }
    }
    impl WidgetImpl for FieldMonitorNavbarRow {}
    impl ListBoxRowImpl for FieldMonitorNavbarRow {}

    impl FieldMonitorNavbarRow {
        fn set_content(&self, content: Option<gtk::Widget>) {
            match content {
                None => self.unset_content(),
                Some(content) => {
                    if self.content.borrow().as_ref() == Some(&content) {
                        return;
                    }
                    self.unset_content();
                    self.do_set_content(content);
                    self.obj().queue_resize();
                }
            }
        }

        fn unset_content(&self) {
            self.content.take();
            let slt_brw = self.slot_middle.borrow();
            let slt_ref = slt_brw.as_ref().unwrap();
            while let Some(child) = slt_ref.last_child() {
                child.unparent();
            }
        }

        fn do_set_content(&self, content: gtk::Widget) {
            let slt_brw = self.slot_middle.borrow();
            let slt_ref = slt_brw.as_ref().unwrap();
            slt_ref.append(&content);
            self.content.replace(Some(content));
        }
    }
}

glib::wrapper! {
    pub struct FieldMonitorNavbarRow(ObjectSubclass<imp::FieldMonitorNavbarRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible, gtk::Actionable;
}

impl FieldMonitorNavbarRow {
    pub fn add_prefix(&self, widget: &impl IsA<gtk::Widget>) {
        let slt_brw = self.imp().slot_left.borrow();
        let slt_ref = slt_brw.as_ref().unwrap();
        slt_ref.append(widget);
    }

    pub fn add_suffix(&self, widget: &impl IsA<gtk::Widget>) {
        let slt_brw = self.imp().slot_right.borrow();
        let slt_ref = slt_brw.as_ref().unwrap();
        slt_ref.append(widget);
    }

    pub fn add_row_action(&self, action_name: &str, cb: impl Fn(Self) + 'static) {
        let mut action_group_brw = self.imp().action_group.borrow_mut();
        let action_group = action_group_brw.get_or_insert_with(|| {
            let group = gio::SimpleActionGroup::new();
            self.insert_action_group("row", Some(&group));
            group
        });

        let action = gio::SimpleAction::new(action_name, None);
        action.connect_activate(glib::clone!(
            #[weak(rename_to=slf)]
            self,
            move |_, _| cb(slf)
        ));

        action_group.add_action(&action)
    }

    // This is all not great 🥲
    pub fn add_context_menu(&self, popover: impl IsA<gtk::PopoverMenu>) {
        let popover = popover.upcast();

        self.update_property(&[gtk::accessible::Property::HasPopup(true)]);

        let gesture_ctrl = gtk::GestureClick::builder()
            .button(MOUSE_RIGHT_BUTTON)
            .build();
        gesture_ctrl.connect_released(glib::clone!(
            #[weak(rename_to=slf)]
            self,
            #[weak]
            popover,
            move |_, _, x, y| slf.show_context_menu(popover, x as _, y as _)
        ));
        self.add_controller(gesture_ctrl);

        let group = gio::SimpleActionGroup::new();
        let action = gio::SimpleAction::new("popup", None);
        action.connect_activate(glib::clone!(
            #[weak(rename_to=slf)]
            self,
            #[weak]
            popover,
            move |_, _| {
                slf.show_context_menu(
                    popover,
                    slf.width() / 2,
                    ((slf.height() as f64) * 0.75) as _,
                );
            }
        ));
        group.add_action(&action);
        self.insert_action_group("menu", Some(&group));
    }

    fn show_context_menu(&self, popover: gtk::PopoverMenu, x: i32, y: i32) {
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x, y, 0, 0)));
        popover.unparent();
        popover.set_parent(self);
        popover.popup();

        self.add_css_class("has-open-popup");
        popover.connect_closed(glib::clone!(
            #[weak(rename_to=slf)]
            self,
            move |_popover| {
                slf.remove_css_class("has-open-popup");
            }
        ));
    }
}
