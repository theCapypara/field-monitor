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
use std::iter;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::anyhow;
use gettextrs::gettext;
use gtk::glib;
use log::{debug, warn};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use parking_lot::Mutex;
use uuid::Uuid;
use vte::prelude::*;

use field_monitor_vte_driver_lib::dbus_server::VtePtyProcMon;
use field_monitor_vte_driver_lib::DBUS_KEY_ENV_VAR;

use crate::adapter::types::{Adapter, AdapterDisplay, AdapterDisplayWidget};
use crate::config::APP_ID;
use crate::connection::ConnectionError;

pub struct VtePtyAdapter {
    connection_id: String,
    server_id: String,
    adapter_id: String,
    command: PathBuf,
    extra_arguments: Vec<String>,
}

impl VtePtyAdapter {
    pub const TAG: &'static str = "vtepty";

    /// Create a new Pty Vte adapter. The command must use field-monitor-vte-driver-lib. See that
    /// crate for more info.
    /// argv is as such: `<command> <dbus-path back to this process service>`
    /// via env variable FM_KEY the process gets a key that can be used with the `extra_arguments`
    /// D-Bus interface method to get the additional argument values.
    pub fn new(
        connection_id: String,
        server_id: String,
        adapter_id: String,
        command: PathBuf,
        extra_arguments: Vec<String>,
    ) -> Self {
        VtePtyAdapter {
            connection_id,
            server_id,
            adapter_id,
            command,
            extra_arguments,
        }
    }
}

impl Adapter for VtePtyAdapter {
    #[allow(clippy::await_holding_lock)] // this is fine here, we do not re-enter
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
    ) -> Box<dyn AdapterDisplay> {
        let vte = vte::Terminal::builder()
            .cursor_blink_mode(vte::CursorBlinkMode::On)
            .build();

        let child_pid: Arc<Mutex<Option<Pid>>> = Arc::default();
        let child_pid_cln = child_pid.clone();

        glib::spawn_future_local(glib::clone!(
            #[strong]
            vte,
            async move {
                // This is not cryptographically safe, but it's not going to be anyway: we pass
                // it to the environment, which isn't secret. This is good enough to at least
                // somewhat obscure the extra arguments in a reasonable way.
                let fm_key = Uuid::now_v7().to_string();

                let dbus_server = match VtePtyProcMon::server(
                    APP_ID,
                    &self.connection_id,
                    &self.server_id,
                    &self.adapter_id,
                    &self.extra_arguments,
                    &fm_key,
                )
                .await
                {
                    Ok(s) => s,
                    Err(err) => {
                        on_disconnected(Err(ConnectionError::General(
                            Some(gettext("Internal error while trying to build terminal.")),
                            err.into(),
                        )));
                        return;
                    }
                };
                let dbus_conn_name = dbus_server.name().to_string();
                let command = self.command.as_os_str().to_string_lossy();

                let argv = [&*command, &*dbus_conn_name];

                let env_owned = glib::environ()
                    .into_iter()
                    .filter_map(|s| s.into_string().ok())
                    .chain(iter::once(format!("{}={}", DBUS_KEY_ENV_VAR, fm_key)))
                    .collect::<Vec<_>>();
                let envv = env_owned.iter().map(Deref::deref).collect::<Vec<_>>();

                match vte
                    .spawn_future(
                        vte::PtyFlags::DEFAULT,
                        None,
                        &argv,
                        &envv,
                        glib::SpawnFlags::DEFAULT,
                        || {},
                        -1,
                    )
                    .await
                {
                    Ok(pid) => {
                        child_pid_cln.lock().replace(Pid::from_raw(pid.0));
                        debug!("pty pid: {pid:?}");
                        on_connected();
                        let dbus_server_arc = Arc::new(Mutex::new(Some(dbus_server)));
                        vte.connect_child_exited(glib::clone!(
                            #[strong]
                            dbus_server_arc,
                            #[strong]
                            on_disconnected,
                            move |_, code| {
                                *child_pid_cln.lock() = None;
                                let dbus_server_guard = dbus_server_arc.lock();
                                // if this is None then child-exited was somehow called more than once?
                                if dbus_server_guard.is_some() {
                                    let end_result = dbus_server_guard
                                        .as_ref()
                                        .unwrap()
                                        .result()
                                        .lock()
                                        .take()
                                        .unwrap_or_else(|| {
                                            Err("(process did not specify result)".to_string())
                                        })
                                        .map_err(|err| {
                                            ConnectionError::General(None, anyhow!("{}", err))
                                        });
                                    drop(dbus_server_guard);
                                    debug!("pty child exited: {code}. Result: {end_result:?}");
                                    let dbus_server_arc_cln = dbus_server_arc.clone();
                                    glib::spawn_future_local(Box::pin(async move {
                                        if let Some(dbus_server) = dbus_server_arc_cln.lock().take()
                                        {
                                            dbus_server.close().await.ok();
                                        }
                                    }));
                                    on_disconnected(end_result.map(|_| ()));
                                }
                            }
                        ));
                    }
                    Err(err) => {
                        on_disconnected(Err(ConnectionError::General(
                            Some(gettext("Internal error while trying to build terminal.")),
                            err.into(),
                        )));
                    }
                }
            }
        ));

        Box::new(VtePtyAdapterDisplay(vte, child_pid))
    }
}

pub struct VtePtyAdapterDisplay(vte::Terminal, Arc<Mutex<Option<Pid>>>);

impl AdapterDisplay for VtePtyAdapterDisplay {
    fn widget(&self) -> AdapterDisplayWidget {
        AdapterDisplayWidget::Vte(self.0.clone())
    }

    fn close(&self) {
        let pty_pid_ctr_guard = self.1.lock();
        if let Some(pty_pid) = &*pty_pid_ctr_guard {
            if let Err(e) = kill(*pty_pid, Signal::SIGKILL) {
                warn!("kill pty subprocess failed: {e}");
            } else {
                debug!("killed pty subprocess");
            }
        }
    }
}

impl Drop for VtePtyAdapterDisplay {
    fn drop(&mut self) {
        self.close()
    }
}
