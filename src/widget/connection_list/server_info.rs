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
//! Functionality to periodically update and display information about a server (either in a
//! row or the title server of a group)

mod icon;

pub use self::icon::ServerInfoIcon;
use crate::widget::connection_list::DEFAULT_GENERIC_ICON;
use crate::widget::connection_list::server_actions::FieldMonitorServerActions;
use crate::widget::connection_list::server_info::icon::IsOnline;
use crate::widget::window::FieldMonitorWindow;
use adw::prelude::*;
use futures::future::LocalBoxFuture;
use glib::object::{Cast, IsA, ObjectType};
use glib::{ControlFlow, WeakRef, timeout_future};
use libfieldmonitor::connection::{IconSpec, ServerConnection, ServerMetadata};
use log::trace;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub struct ServerInfoUpdater<T>
where
    T: ObjectType,
{
    target: WeakRef<T>,
    server: Rc<Box<dyn ServerConnection>>,
    full_path: Rc<Vec<String>>,
}

impl<T> ServerInfoUpdater<T>
where
    T: ServerInfoWidget + ObjectType,
{
    pub fn start(target: WeakRef<T>, server: Rc<Box<dyn ServerConnection>>, full_path: &[String]) {
        let full_path = Rc::new(full_path.to_vec());
        let slf = ServerInfoUpdater {
            target,
            server,
            full_path,
        };

        slf.run_updater(Self::update_server_info, Duration::from_secs(3));
    }

    fn update_server_info(
        target: T,
        server: Rc<Box<dyn ServerConnection>>,
        path: Rc<Vec<String>>,
        call_count: u64,
    ) -> LocalBoxFuture<'static, bool> {
        // TODO: Ideally we really shouldn't spawn a future per row, but instead have one
        //       update loop for all rows, where we also only check once if the list is visible.
        Box::pin(async move {
            let window = target.window();
            if window
                .map(|w| !w.is_connection_view_visible())
                .unwrap_or_default()
            {
                let metadata = server.metadata().await;
                let container = target.get_icon_container();

                // -- every three seconds
                // Check online status
                let new_status = match metadata.is_online {
                    Some(true) => IsOnline::Online,
                    Some(false) => IsOnline::Offline,
                    None => IsOnline::Unknown,
                };
                let old_status = container.status();
                let online_status_changed = old_status != IsOnline::Unknown
                    && new_status != IsOnline::Unknown
                    && old_status != new_status;
                if online_status_changed {
                    trace!("{}: online status changed", path.join("/"));
                }

                if call_count.is_multiple_of(40) {
                    // -- every two minutes
                    trace!("{}: update icon", path.join("/"));
                    Self::update_icon(&target, new_status, &metadata);
                }
                if call_count.is_multiple_of(20) {
                    // -- once per minute
                    trace!("{}: update title & subtitle", path.join("/"));
                    target.set_server_title(&metadata.title);
                    target.set_server_subtitle(metadata.subtitle.as_deref());
                }
                if online_status_changed || call_count.is_multiple_of(15) {
                    // -- every 45 seconds or if online status changed
                    let path = path.join("/");
                    trace!("{}: update buttons", path);
                    FieldMonitorServerActions::update_server_info_widget(&target, &**server, &path)
                        .await;
                    trace!("{}: update icon status", path);
                    container.set_status(new_status);
                }
                true
            } else {
                // to limit performance impact while connection view is active: delay next update
                timeout_future(Duration::from_secs(10)).await;
                false
            }
        })
    }

    /// Run the update function in a regular interval until the target widget stops to exist.
    fn run_updater<F>(&self, cb: F, duration: Duration)
    where
        F: Fn(
                T,
                Rc<Box<dyn ServerConnection>>,
                Rc<Vec<String>>,
                u64,
            ) -> LocalBoxFuture<'static, bool>
            + 'static,
    {
        let mut flow = ControlFlow::Continue;
        let target = self.target.clone();
        let server = self.server.clone();
        let full_path = self.full_path.clone();
        let update_call_count = AtomicU64::new(0);

        glib::spawn_future_local(async move {
            while flow == ControlFlow::Continue {
                match target.upgrade() {
                    None => {
                        flow = ControlFlow::Break;
                    }
                    Some(target) => {
                        let cb_result = cb(
                            target,
                            server.clone(),
                            full_path.clone(),
                            update_call_count.load(Ordering::Relaxed),
                        )
                        .await;
                        if cb_result {
                            update_call_count.fetch_add(1, Ordering::Relaxed);
                        } else {
                            // if we didn't run an update we reset the counter, to cause a full update
                            // next time
                            update_call_count.store(0, Ordering::Relaxed);
                        }
                        timeout_future(duration).await;
                    }
                }
            }
        });
    }

    fn update_icon(target: &T, status: IsOnline, metadata: &ServerMetadata) {
        let container = target.get_icon_container();
        container.set_status(status);

        // If true, we were able to just set an icon name in the existing widget, if false
        // we have to replace the child
        let mut simple_update = false;

        if let Some(image) = container.child().and_downcast::<gtk::Image>() {
            let icon_name = match &metadata.icon {
                IconSpec::Default => Some(DEFAULT_GENERIC_ICON),
                IconSpec::Named(name) => Some(name.as_ref()),
                _ => None,
            };
            if let Some(icon_name) = icon_name {
                simple_update = true;
                image.set_icon_name(Some(icon_name));
            };
        }

        if !simple_update {
            let wdg = match &metadata.icon {
                IconSpec::Default => gtk::Image::builder()
                    .icon_name(DEFAULT_GENERIC_ICON)
                    .build()
                    .upcast(),
                IconSpec::None => gtk::Box::builder().width_request(16).build().upcast(),
                IconSpec::Named(name) => gtk::Image::builder()
                    .icon_name(name.as_ref())
                    .build()
                    .upcast(),
                IconSpec::Custom(factory) => factory(metadata),
            };

            container.set_child(wdg);
        }
    }
}

pub trait ServerInfoWidget {
    fn window(&self) -> Option<FieldMonitorWindow>;
    fn set_server_title(&self, title: &str);
    fn set_server_subtitle(&self, subtitle: Option<&str>);
    fn get_icon_container(&self) -> ServerInfoIcon;
    fn get_actions_container(&self) -> adw::Bin;
    fn get_row(&self) -> Option<&impl IsA<adw::ActionRow>>;
}
