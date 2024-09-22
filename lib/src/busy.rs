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

use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct BusyStack(Rc<RefCell<usize>>, Rc<Cell<bool>>, Rc<Box<dyn Fn()>>);

impl BusyStack {
    pub fn new(backend: Rc<Cell<bool>>, notify: Box<dyn Fn()>) -> BusyStack {
        backend.replace(false);
        Self(Rc::new(RefCell::new(0)), backend, Rc::new(notify))
    }

    pub fn busy(&self) -> BusyGuard {
        let mut count = self.0.borrow_mut();
        *count += 1;
        let old = self.1.replace(true);
        if !old {
            self.2();
        }
        BusyGuard(self.0.clone(), self.1.clone(), self.2.clone())
    }
}

pub struct BusyGuard(Rc<RefCell<usize>>, Rc<Cell<bool>>, Rc<Box<dyn Fn()>>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        let mut count = self.0.borrow_mut();
        *count = count.saturating_sub(1);
        if *count == 0 {
            let old = self.1.replace(false);
            if old {
                self.2();
            }
        }
    }
}
