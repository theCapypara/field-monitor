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

//! A bin widget that plays an opacity pulse animation when it becomes visible.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use log::trace;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FieldMonitorPulseAnimBin;

    #[glib::object_subclass]
    impl ObjectSubclass for FieldMonitorPulseAnimBin {
        const NAME: &'static str = "FieldMonitorPulseAnimBin";
        type Type = super::FieldMonitorPulseAnimBin;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for FieldMonitorPulseAnimBin {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().connect_visible_notify(|bin| {
                let visible = bin.is_visible();
                trace!("FieldMonitorPulseAnimBin set visible {visible}");
                if visible {
                    let target = adw::PropertyAnimationTarget::new(bin, "opacity");
                    let animation = adw::TimedAnimation::new(bin, 1.0, 0.0, 400, target);
                    animation.set_easing(adw::Easing::EaseInOutCubic);
                    animation.set_repeat_count(2);
                    animation.set_alternate(true);
                    animation.play();
                }
            });
        }
    }

    impl WidgetImpl for FieldMonitorPulseAnimBin {}
    impl BinImpl for FieldMonitorPulseAnimBin {}
}

glib::wrapper! {
    pub struct FieldMonitorPulseAnimBin(ObjectSubclass<imp::FieldMonitorPulseAnimBin>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}
