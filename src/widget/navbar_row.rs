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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorNavbarRow {
        const NAME: &'static str = "FieldMonitorNavbarRow";
        type Type = super::FieldMonitorNavbarRow;
        type ParentType = gtk::ListBoxRow;
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
}
