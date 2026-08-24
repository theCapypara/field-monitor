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
pub use debug_info::debug_info;
use logforth::append;
use logforth::filter::rustlog::RustLogFilterBuilder;
use logforth::layout;
use std::io;
use std::io::IsTerminal;

/// Initialize logging
pub fn init() {
    #[cfg(feature = "devel")] // Setup debug logging
    {
        // SAFETY: This is generally safe to call with correct boolean arguments.
        unsafe {
            rdw_vnc::gvnc::ffi::vnc_util_set_debug(glib::ffi::GTRUE);
        }
    }
    glib::log_set_default_handler(glib::rust_log_handler);
    glib::log_set_writer_func(glib::rust_log_writer);

    logforth::starter_log::builder()
        // stderr: log what RUST_LOG specifies
        .dispatch(|d| {
            d.filter(RustLogFilterBuilder::from_default_env().build())
                .append(append::Stderr::default().with_layout(make_stderr_text_layout()))
        })
        // debug info: log all debug and higher
        .dispatch(|d| {
            // note: with the `prod` feature this effectively becomes `Info`
            d.filter(logforth::record::LevelFilter::MoreSevereEqual(
                logforth::record::Level::Debug,
            ))
            .append(debug_info::DebugInfoAppender::new_with_layout(
                layout::JsonLayout::default(),
            ))
        })
        .apply();
}

fn make_stderr_text_layout() -> layout::TextLayout {
    if !io::stderr().is_terminal() {
        layout::TextLayout::default().no_color()
    } else {
        layout::TextLayout::default()
    }
}

mod debug_info {
    use itertools::Itertools;
    use logforth::append;
    use logforth::layout;
    use logforth::record;
    use std::cell::Cell;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    thread_local! {
        static CURRENTLY_COLLECTING: Cell<bool> = const { Cell::new(false) };
    }
    static DEBUG_INFO: Mutex<(usize, VecDeque<Vec<u8>>)> = Mutex::new((0, VecDeque::new()));

    /// Get the currently collected debug information.
    pub fn debug_info() -> String {
        CURRENTLY_COLLECTING.set(true);
        let (_, buf) = &*DEBUG_INFO
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let output = buf.iter().map(|v| String::from_utf8_lossy(v)).join("\n");
        CURRENTLY_COLLECTING.set(false);
        output
    }

    const MAX_BUF_BYTE_SIZE: usize = 10_000_000; // 10mb

    #[derive(Debug)]
    pub struct DebugInfoAppender {
        layout: Box<dyn layout::Layout>,
    }

    impl DebugInfoAppender {
        pub fn new_with_layout(layout: impl Into<Box<dyn layout::Layout>>) -> Self {
            Self {
                layout: layout.into(),
            }
        }
    }

    impl append::Append for DebugInfoAppender {
        fn append(
            &self,
            record: &record::Record,
            diags: &[Box<dyn logforth::Diagnostic>],
        ) -> Result<(), logforth::Error> {
            // we can't process log messages while currently collecting, since
            // that holds the lock
            if CURRENTLY_COLLECTING.get() {
                return if cfg!(debug_assertions) {
                    Err(logforth::Error::new(
                        "tried to log while collecting debug info",
                    ))
                } else {
                    Ok(())
                };
            }
            let bytes = self.layout.format(record, diags)?;

            // Should be fast enough:
            // even with excessive logging this Mutex will rarely be contended.
            let (buf_size, buf) = &mut *DEBUG_INFO
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            let by_len = bytes.len();
            if by_len > MAX_BUF_BYTE_SIZE {
                // we discard log messages that are this big
                return Ok(());
            }
            let mut si = *buf_size + by_len;
            while si > MAX_BUF_BYTE_SIZE {
                let front = buf.pop_front();
                si -= front.unwrap_or_default().len();
            }
            buf.push_back(bytes);
            *buf_size = si;

            Ok(())
        }

        fn flush(&self) -> Result<(), logforth::Error> {
            Ok(())
        }
    }
}
