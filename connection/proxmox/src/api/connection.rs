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
use crate::api;
use crate::api::cache::InfoFetcher;
use crate::api::node::ProxmoxNode;
use crate::preferences::ProxmoxConfiguration;
use anyhow::anyhow;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use http::Uri;
use libfieldmonitor::connection::{
    Actionable, Connection, ConnectionConfiguration, ConnectionError, ConnectionMetadata,
    ConnectionMetadataBuilder, ConnectionResult, IconSpec, ServerMap, ServerMapSend,
};
use libfieldmonitor::tokiort::run_on_tokio;
use proxmox_api::ProxmoxApiClient;
use secure_string::SecureString;
use std::mem::transmute;
use std::num::NonZeroU32;
use std::str::FromStr;
use std::sync::Arc;

pub struct ProxmoxConnection {
    connection_id: String,
    title: String,
    info_fetcher: Arc<InfoFetcher>,
}

impl ProxmoxConnection {
    pub(super) async fn connect(config: ConnectionConfiguration) -> ConnectionResult<Self> {
        let authority = format!(
            "{}:{}",
            config.hostname().unwrap_or_default(),
            config.port().map(NonZeroU32::get).unwrap_or(8006)
        );

        let api_root = Uri::builder()
            .scheme("https")
            .authority(authority)
            .path_and_query("/api2/json")
            .build()
            .map_err(|err| {
                ConnectionError::General(
                    Some(gettext(
                        "Was unable to build a valid URL to connect to. Check your settings.",
                    )),
                    anyhow!(err),
                )
            })?;

        let pass = config
            .password_or_apikey()
            .await
            .map_err(|err| {
                ConnectionError::General(
                    Some(gettext(
                        "Failed to retrieve API Key or Password from secrets service.",
                    )),
                    anyhow!(err),
                )
            })?
            .unwrap_or_else(|| SecureString::from_str("").unwrap());

        let client = if config.use_apikey() {
            ProxmoxApiClient::connect_with_apikey(
                &api_root,
                config.tokenid().unwrap_or_default(),
                pass,
                config.ignore_ssl_cert_error(),
            )
            .await
            .map_err(api::map_proxmox_error)
        } else {
            ProxmoxApiClient::connect_with_ticket(
                &api_root,
                config.username().unwrap_or_default(),
                pass,
                config.ignore_ssl_cert_error(),
            )
            .await
            .map_err(api::map_proxmox_error)
        }?;

        Ok(Self {
            connection_id: config.id().to_string(),
            title: config.title().unwrap_or_default().to_string(),
            info_fetcher: Arc::new(InfoFetcher::new(Arc::new(client))),
        })
    }
}

impl Actionable for ProxmoxConnection {}

impl Connection for ProxmoxConnection {
    fn metadata(&self) -> LocalBoxFuture<ConnectionMetadata> {
        Box::pin(async move {
            ConnectionMetadataBuilder::default()
                .title(self.title.clone())
                .icon(IconSpec::Named("connection-proxmox-symbolic".into()))
                .build()
                .unwrap()
        })
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            let connection_id = self.connection_id.clone();
            let info_fetcher = self.info_fetcher.clone();
            let map = run_on_tokio::<_, _, ConnectionError>(async move {
                let mut server_map = ServerMapSend::default();

                for node in info_fetcher.nodes().await? {
                    server_map.insert(
                        node.node.to_string().into(),
                        Box::new(ProxmoxNode {
                            info_fetcher: info_fetcher.clone(),
                            connection_id: connection_id.clone(),
                            id: node.node.clone(),
                        }),
                    );
                }

                Ok(server_map)
            })
            .await?;

            // TODO: Is this actually safe?
            let map_cast: ServerMap = unsafe { transmute(map) };

            Ok(map_cast)
        })
    }
}
