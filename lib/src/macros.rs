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

#[macro_export]
macro_rules! impl_simple_macro_param_spec {
    ($name:ty, $base:tt) => {
        impl ::glib::HasParamSpec for $name {
            type ParamSpec = <$base as ::glib::HasParamSpec>::ParamSpec;
            type SetValue = Self;
            type BuilderFn = <$base as ::glib::HasParamSpec>::BuilderFn;

            fn param_spec_builder() -> Self::BuilderFn {
                Self::ParamSpec::builder
            }
        }

        impl ::glib::value::ToValue for $name {
            fn to_value(&self) -> ::glib::Value {
                $base::to_value(&(*self as $base))
            }

            fn value_type(&self) -> ::glib::Type {
                $base::static_type()
            }
        }

        unsafe impl<'a> ::glib::value::FromValue<'a> for $name {
            type Checker = <$base as ::glib::value::FromValue<'a>>::Checker;

            unsafe fn from_value(value: &'a ::glib::Value) -> Self {
                Self::try_from($base::from_value(value)).unwrap_or_default()
            }
        }
    };
}
