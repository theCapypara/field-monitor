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
use crate::adapter::types::{Adapter, AdapterDisplay, AdapterDisplayWidget, NullAdapterDisplay};
use crate::adapter::usbredir::FieldMonitorUsbRedirAdapter;
use crate::adapter::usbredir::qemu_dbus::FieldMonitorUsbRedirQemuDbus;
use crate::cert_security::{VerifyTls, VerifyTlsResponse};
use crate::connection::ConnectionError;
use futures::future::LocalBoxFuture;
use gtk::prelude::*;
use log::{debug, error, warn};
use rdw_qemu;
use rdw_qemu::qemu_display;
use std::cell::RefCell;
use std::future;
use std::rc::Rc;
use thiserror::Error;
use zbus;

pub struct QemuDbusAdapter {
    bus_conn: zbus::Connection,
    vm_name: Option<String>,
}

impl QemuDbusAdapter {
    pub const TAG: &'static str = "qemu-dbus";

    /// Init the adapter. If `vm_name` is None, no lookup will be performed - used for p2p
    /// connections.
    pub fn new(bus_conn: zbus::Connection, vm_name: Option<String>) -> Self {
        Self { bus_conn, vm_name }
    }
}

impl Adapter for QemuDbusAdapter {
    fn create_and_connect_display(
        self: Box<Self>,
        on_connected: Rc<dyn Fn()>,
        on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
        _verify_tls: Rc<dyn Fn(VerifyTls) -> VerifyTlsResponse>,
    ) -> LocalBoxFuture<'static, Box<dyn AdapterDisplay>> {
        Box::pin(async move {
            debug!("creating qemu-dbus adapter");

            async fn try_connect(
                conn: zbus::Connection,
                vm_name: Option<String>,
                on_disconnected: Rc<dyn Fn(Result<(), ConnectionError>)>,
            ) -> Result<Box<dyn AdapterDisplay>, QemuDbusConnectError> {
                debug!(
                    "trying to connect to qemu via dbus. looking for {:?} @ {}",
                    vm_name,
                    conn.server_guid()
                );
                let dest = match vm_name.as_deref() {
                    None => None,
                    Some(vm_name) => qemu_display::Display::lookup(&conn, false, Some(vm_name))
                        .await
                        .map_err(|err| {
                            warn!("failed to lookup display: {}", err);
                            QemuDbusConnectError::Lookup(err)
                        })?,
                };

                let qemu_display =
                    qemu_display::Display::new(&conn, dest)
                        .await
                        .map_err(|err| {
                            warn!("failed to create qemu_display: {}", err);
                            QemuDbusConnectError::InitDisplay(err)
                        })?;

                let rdw_display = rdw_qemu::Display::new_with_qemu_display(
                    &qemu_display,
                    Some(move |r: Result<(), qemu_display::Error>| {
                        on_disconnected(
                            r.map_err(|err| QemuDbusConnectError::Communication(err).into()),
                        )
                    }),
                )
                .await
                .map_err(|err| {
                    warn!("failed to construct Console: {}", err);
                    QemuDbusConnectError::InitConsole(err)
                })?;

                Ok(Box::new(QemuDbusAdapterDisplay(RefCell::new(Some(
                    DisplayInner {
                        connection: Some(conn),
                        qemu_display,
                        rdw_display,
                    },
                )))))
            }

            // Since rdw4_qemu::Display needs the Console to even be constructed,
            // we do the entire connection process here and schedule a call to on_connected
            // right before returning the widget.
            match try_connect(self.bus_conn, self.vm_name, on_disconnected.clone()).await {
                Ok(adapter) => {
                    glib::spawn_future_local(async move { on_connected() });
                    adapter
                }
                Err(e) => {
                    glib::spawn_future_local(async move { on_disconnected(Err(e.into())) });
                    Box::new(NullAdapterDisplay)
                }
            }
        })
    }
}

#[derive(Debug, Error)]
enum QemuDbusConnectError {
    #[error("failed to lookup VM on D-bus bus: {0}")]
    Lookup(qemu_display::Error),
    #[error("failed to initialize D-Bus display: {0}")]
    InitDisplay(qemu_display::Error),
    #[error("failed to initialize D-Bus console: {0}")]
    InitConsole(qemu_display::Error),
    #[error("failed to communicate via the bus: {0}")]
    Communication(qemu_display::Error),
}

impl From<QemuDbusConnectError> for ConnectionError {
    fn from(value: QemuDbusConnectError) -> Self {
        ConnectionError::General(Some(format!("{}", value)), value.into())
    }
}

pub struct QemuDbusAdapterDisplay(RefCell<Option<DisplayInner<'static>>>);

impl AdapterDisplay for QemuDbusAdapterDisplay {
    fn widget(&self) -> AdapterDisplayWidget {
        match &*self.0.borrow() {
            None => {
                error!("bug: qemu-dbus adapter (already) closed but widget requested");
                NullAdapterDisplay.widget()
            }
            Some(display) => AdapterDisplayWidget::Rdw(display.rdw_display.clone().upcast()),
        }
    }

    fn close(&self) {
        debug!("closing qemu-dbus session");
        self.0.borrow_mut().take();
    }

    fn create_usb_redir_adapter(
        &'_ self,
    ) -> LocalBoxFuture<'_, Option<FieldMonitorUsbRedirAdapter>> {
        self.0
            .borrow()
            .as_ref()
            .map(
                |inner| -> LocalBoxFuture<'_, Option<FieldMonitorUsbRedirAdapter>> {
                    let qemu_display = inner.qemu_display.clone();
                    Box::pin(async move {
                        let result = FieldMonitorUsbRedirQemuDbus::new(
                            qemu_display.usbredir_chardevs().await,
                        )
                        .await;
                        match result {
                            Ok(w) => Some(w.upcast()),
                            Err(err) => {
                                error!("failed to init usb redirection for QEMU: {err}");
                                None
                            }
                        }
                    })
                },
            )
            .unwrap_or_else(|| Box::pin(future::ready(None)))
    }
}

struct DisplayInner<'a> {
    connection: Option<zbus::Connection>,
    qemu_display: qemu_display::Display<'a>,
    rdw_display: rdw_qemu::Display,
}

impl Drop for DisplayInner<'_> {
    fn drop(&mut self) {
        debug!("dropping QemuDbusAdapterDisplay/DisplayInner");
        if let Some(connection) = self.connection.take() {
            glib::spawn_future_local(async move {
                let result = connection.close().await;
                debug!("closed qemu-dbus connection: {result:?}");
            });
        }
    }
}
