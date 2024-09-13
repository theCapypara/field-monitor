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
use std::fs::read_dir;
use std::path::PathBuf;

use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::{gio, glib};
use gtk::prelude::*;
use log::info;

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};

use crate::config::APP_ID;

use self::application::FieldMonitorApplication;

mod application;
mod config;
mod connection;
mod i18n;
mod secrets;
mod widget;

fn main() -> glib::ExitCode {
    glib::log_set_default_handler(glib::rust_log_handler);
    pretty_env_logger::init_timed();

    // Set up gettext translations
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    // Load resources
    for file in read_dir(PathBuf::from(PKGDATADIR)).expect("Failed to read resources dir") {
        let file = file.expect("Failed to read resources dir");
        if file
            .path()
            .file_name()
            .expect("Failed to read resource filename")
            .to_string_lossy()
            .ends_with(".gresource")
        {
            let resources = gio::Resource::load(file.path()).expect("Could not load resources");
            gio::resources_register(&resources);
        }
    }

    // Create a new GtkApplication. The application manages our main loop,
    // application windows, integration with the window manager/compositor, and
    // desktop features such as file opening and single-instance applications.
    let app = FieldMonitorApplication::new(APP_ID, &gio::ApplicationFlags::empty());

    // Accelerators
    app.set_accels_for_action("win.toggle-fullscreen", &["F11"]);

    // Run the application. This function will block until the application
    // exits. Upon return, we have our exit code to return to the shell. (This
    // is the code you see when you do `echo $?` after running a command in a
    // terminal.
    info!("started Field Monitor");
    app.run()
}
