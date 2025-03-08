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
#[macro_export]
macro_rules! debug {
    ($client:expr, $($arg:tt)+) => {
        #[cfg(debug_assertions)]
        {
            $client.log_debug(&format!($($arg)+)).await;
        }
    }
}

#[macro_export]
macro_rules! warn {
    ($client:expr, $($arg:tt)+) => {
        $client.log_warn(&format!($($arg)+)).await;
    }
}

#[macro_export]
macro_rules! error {
    ($client:expr, $($arg:tt)+) => {
        $client.log_error(&format!($($arg)+)).await;
    }
}

// the sync macros require Tokio to be available in the calling crate.

#[macro_export]
macro_rules! debug_sync {
    ($client:expr, $($arg:tt)+) => {
        #[cfg(debug_assertions)]
        {
            let v = format!($($arg)+);
            let client_cln = $client.clone();
            ::tokio::runtime::Handle::current().block_on(async move { client_cln.log_debug(&v).await });
        }
    }
}

#[macro_export]
macro_rules! warn_sync {
    ($client:expr, $($arg:tt)+) => {
        {
            let v = format!($($arg)+);
            let client_cln = $client.clone();
            ::tokio::runtime::Handle::current().block_on(async move { client_cln.log_warn(&v).await });
        }
    }
}

#[macro_export]
macro_rules! error_sync {
    ($client:expr, $($arg:tt)+) => {
        {
            let v = format!($($arg)+);
            let client_cln = $client.clone();
            ::tokio::runtime::Handle::current().block_on(async move { client_cln.log_error(&v).await });
        }
    }
}

#[macro_export]
macro_rules! args {
    ($client:expr => ($($var:ident),+ $(,)?)) => {
        let args = $client.args();
        let mut argsi = args.iter();
        $(
            let $var = *argsi.next().as_ref().expect("not enough arguments supplied");
        )+
    }
}
