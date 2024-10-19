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
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::future::BoxFuture;
use futures::lock::Mutex;
use http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri};
use log::{debug, warn};
use reqwest::{ClientBuilder, RequestBuilder, Response};
use secure_string::SecureString;
use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;

pub use crate::datatypes::*;

mod datatypes;

#[derive(Debug, Error)]
pub enum Error {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
    #[error("the value provided for an identifier is invalid")]
    InvalidIdValue,
    #[error("authentication failed")]
    AuthFailed,
    #[error("API returned no data")]
    MissingData,
    #[error("API failed with status {0}")]
    ApiUnknown(StatusCode),
    #[error("API failed with status {0}: {1}")]
    Api(StatusCode, String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
struct Client {
    client: reqwest::Client,
    root: String, // ends with /
}

impl Client {
    fn new(root: &Uri, ignore_ssl_errors: bool) -> Result<Self> {
        let mut root = root.to_string();
        if !root.ends_with('/') {
            root = format!("{root}/");
        }
        Ok(Self {
            client: ClientBuilder::new()
                .danger_accept_invalid_certs(ignore_ssl_errors)
                .build()?,
            root,
        })
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.client
            .request(method, format!("{}{}", self.root, path))
    }
}

struct AuthHeaders<'a> {
    auth_header: Cow<'a, str>,
    extra_headers: HeaderMap,
}

struct TicketProvider {
    client: Client,
    user: String,
    password: SecureString,
    just_reauth: AtomicBool,
    current_ticket: Arc<Mutex<Option<Ticket>>>,
}

impl TicketProvider {
    async fn reauth(&self) -> Result<()> {
        debug!("re-issuing ticket");
        self.just_reauth.store(true, Ordering::Release);
        self.current_ticket.lock().await.replace(
            self.client
                .request(Method::POST, "access/ticket")
                .form(&[
                    ("username", &*self.user),
                    ("password", self.password.unsecure()),
                ])
                .send()
                .await?
                .json::<Wrapper<Ticket>>()
                .await?
                .data
                .ok_or(Error::MissingData)?,
        );
        Ok(())
    }
}

struct ApikeyProvider {
    tokenid: String,
    apikey: SecureString,
}

enum DoAfterAuthRetry {
    Retry,
    Fail,
}

trait ApiAccessProvider {
    fn provide_auth_headers(&self) -> BoxFuture<Result<AuthHeaders>>;
    fn auth_success(&self) {}
    fn failed_auth(&self) -> BoxFuture<DoAfterAuthRetry>;
}

impl ApiAccessProvider for ApikeyProvider {
    fn provide_auth_headers(&self) -> BoxFuture<Result<AuthHeaders>> {
        Box::pin(async move {
            Ok(AuthHeaders {
                auth_header: format!("PVEAPIToken={}={}", self.tokenid, self.apikey.unsecure())
                    .into(),
                extra_headers: HeaderMap::new(),
            })
        })
    }

    fn failed_auth(&self) -> BoxFuture<DoAfterAuthRetry> {
        Box::pin(async move { DoAfterAuthRetry::Fail })
    }
}

impl ApiAccessProvider for TicketProvider {
    fn provide_auth_headers(&self) -> BoxFuture<Result<AuthHeaders>> {
        let current_ticket = self.current_ticket.clone();
        Box::pin(async move {
            let lock = current_ticket.lock().await;
            if let Some(ticket) = &*lock {
                let mut extra_headers = HeaderMap::with_capacity(1);
                extra_headers.insert(
                    "CSRFPreventionToken",
                    HeaderValue::from_str(&ticket.csrf_prevention_token)?,
                );
                Ok(AuthHeaders {
                    auth_header: format!("PVEAuthCookie={}", ticket.ticket).into(),
                    extra_headers,
                })
            } else {
                drop(lock);
                self.reauth().await?;
                self.provide_auth_headers().await
            }
        })
    }

    fn auth_success(&self) {
        self.just_reauth.store(false, Ordering::Release)
    }

    fn failed_auth(&self) -> BoxFuture<DoAfterAuthRetry> {
        Box::pin(async move {
            if self.just_reauth.load(Ordering::Acquire) {
                DoAfterAuthRetry::Fail
            } else {
                match self.reauth().await {
                    Ok(_) => DoAfterAuthRetry::Retry,
                    Err(err) => {
                        warn!("ticket reissuing failed: {err}");
                        DoAfterAuthRetry::Fail
                    }
                }
            }
        })
    }
}

pub struct ProxmoxApiClient {
    client: Client,
    api_access_provider: Box<dyn ApiAccessProvider + Send + Sync>,
}

/// Public API
impl ProxmoxApiClient {
    pub async fn connect_with_apikey(
        root: &Uri,
        tokenid: &str,
        apikey: SecureString,
        ignore_ssl_errors: bool,
    ) -> Result<Self> {
        debug!("creating proxmox client with api key");
        let client = Client::new(root, ignore_ssl_errors)?;
        Ok(Self {
            client: client.clone(),
            api_access_provider: Box::new(ApikeyProvider {
                tokenid: tokenid.to_string(),
                apikey,
            }),
        })
    }

    pub async fn connect_with_ticket(
        root: &Uri,
        user: &str,
        password: SecureString,
        ignore_ssl_errors: bool,
    ) -> Result<Self> {
        debug!("creating proxmox client with username and password");
        let client = Client::new(root, ignore_ssl_errors)?;
        Ok(Self {
            client: client.clone(),
            api_access_provider: Box::new(TicketProvider {
                client,
                user: user.to_string(),
                password,
                just_reauth: Default::default(),
                current_ticket: Arc::new(Default::default()),
            }),
        })
    }

    pub async fn nodes(&self) -> Result<Vec<Node>> {
        self.get_without_params_json("nodes").await
    }

    pub async fn node_lxc(&self, node: &NodeId) -> Result<Vec<LxcVm>> {
        let mut vms: Vec<LxcVm> = self
            .get_without_params_json(&format!("nodes/{}/lxc", node))
            .await?;

        vms.sort_unstable_by_key(|vm| vm.vmid.clone());

        Ok(vms)
    }

    pub async fn node_qemu(&self, node: &NodeId, full: bool) -> Result<Vec<QemuVm>> {
        let mut vms: Vec<QemuVm> = self
            .get_json(
                &format!("nodes/{}/qemu", node),
                &[("full", if full { "1" } else { "0" })],
            )
            .await?;

        vms.sort_unstable_by_key(|vm| vm.vmid.clone());

        Ok(vms)
    }

    pub async fn node_reboot(&self, node: &NodeId) -> Result<()> {
        let response = self
            .post_form(&format!("nodes/{}/status", node), &[("command", "reboot")])
            .await?;
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            Err(Error::ApiUnknown(status))
        } else {
            Ok(())
        }
    }

    pub async fn node_shutdown(&self, node: &NodeId) -> Result<()> {
        let response = self
            .post_form(
                &format!("nodes/{}/status", node),
                &[("command", "shutdown")],
            )
            .await?;
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            Err(Error::ApiUnknown(status))
        } else {
            Ok(())
        }
    }

    pub async fn vm_qemu_status_current(&self, node: &NodeId, vm: &VmId) -> Result<QemuVmStatus> {
        self.get_without_params_json(&format!("nodes/{}/qemu/{}/status/current", node, vm))
            .await
    }

    pub async fn vm_available_console_proxies(
        &self,
        node: &NodeId,
        vm: &VmId,
        vm_type: Option<VmType>,
    ) -> Result<impl AsRef<[VmConsoleProxyType]>> {
        const ALL: [VmConsoleProxyType; 3] = [
            VmConsoleProxyType::Term,
            VmConsoleProxyType::Vnc,
            VmConsoleProxyType::Spice,
        ];

        let vm_type = self.vm_type(node, vm, vm_type).await?;
        Ok(match vm_type {
            VmType::Lxc => &ALL[..],
            VmType::Qemu => {
                if self
                    .vm_qemu_status_current(node, vm)
                    .await?
                    .spice
                    .unwrap_or_default()
                {
                    &ALL[..]
                } else {
                    &[VmConsoleProxyType::Term, VmConsoleProxyType::Vnc][..]
                }
            }
        })
    }

    pub async fn vm_start(
        &self,
        node: &NodeId,
        vm: &VmId,
        vm_type: Option<VmType>,
        input: VmStartInput,
    ) -> Result<String> {
        self.vm_post_status(node, vm, vm_type, "start", input).await
    }

    pub async fn vm_stop(
        &self,
        node: &NodeId,
        vm: &VmId,
        vm_type: Option<VmType>,
        input: VmStopInput,
    ) -> Result<String> {
        self.vm_post_status(node, vm, vm_type, "stop", input).await
    }

    pub async fn qemu_vm_reset(
        &self,
        node: &NodeId,
        vm: &VmId,
        input: VmResetInputQemu,
    ) -> Result<String> {
        self.post_form_json(&format!("nodes/{node}/qemu/{vm}/status/reset"), &input)
            .await
    }

    pub async fn vm_shutdown(
        &self,
        node: &NodeId,
        vm: &VmId,
        vm_type: Option<VmType>,
        input: VmShutdownInput,
    ) -> Result<String> {
        self.vm_post_status(node, vm, vm_type, "shutdown", input)
            .await
    }

    pub async fn vm_reboot(
        &self,
        node: &NodeId,
        vm: &VmId,
        vm_type: Option<VmType>,
        input: VmRebootInput,
    ) -> Result<String> {
        self.vm_post_status(node, vm, vm_type, "reboot", input)
            .await
    }

    pub async fn vm_suspend(
        &self,
        node: &NodeId,
        vm: &VmId,
        vm_type: Option<VmType>,
        input: VmSuspendInput,
    ) -> Result<String> {
        self.vm_post_status(node, vm, vm_type, "suspend", input)
            .await
    }

    pub async fn node_termproxy(
        &self,
        node: &NodeId,
        input: NodeTermproxyInput,
    ) -> Result<Termproxy> {
        self.post_form_json(&format!("nodes/{}/termproxy", node), &input)
            .await
    }

    pub async fn vm_termproxy(
        &self,
        node: &NodeId,
        vm: &VmId,
        vm_type: Option<VmType>,
        input: VmTermproxyInput,
    ) -> Result<(VmType, Termproxy)> {
        let vm_type = self.vm_type(node, vm, vm_type).await?;
        let resp = match vm_type {
            VmType::Lxc => {
                self.post_form_json(
                    &format!("nodes/{node}/lxc/{vm}/termproxy"),
                    &input.into_lxc(),
                )
                .await
            }
            VmType::Qemu => {
                self.post_form_json(
                    &format!("nodes/{node}/qemu/{vm}/termproxy"),
                    &input.into_qemu(),
                )
                .await
            }
        }?;
        Ok((vm_type, resp))
    }

    pub async fn node_spiceshell(
        &self,
        node: &NodeId,
        input: NodeSpiceshellInput,
    ) -> Result<Spiceproxy> {
        self.post_form_json(&format!("nodes/{}/spiceshell", node), &input)
            .await
    }

    pub async fn vm_spiceproxy(
        &self,
        node: &NodeId,
        vm: &VmId,
        vm_type: Option<VmType>,
        input: VmSpiceproxyInput,
    ) -> Result<Spiceproxy> {
        let vm_type = self.vm_type(node, vm, vm_type).await?;
        match vm_type {
            VmType::Lxc => {
                self.post_form_json(&format!("nodes/{node}/lxc/{vm}/spiceproxy"), &input)
                    .await
            }
            VmType::Qemu => {
                self.post_form_json(&format!("nodes/{node}/qemu/{vm}/spiceproxy"), &input)
                    .await
            }
        }
    }

    pub async fn node_vncshell(&self, node: &NodeId, input: NodeVncshellInput) -> Result<Vncproxy> {
        self.post_form_json(&format!("nodes/{}/vncshell", node), &input)
            .await
    }

    pub async fn vm_vncproxy(
        &self,
        node: &NodeId,
        vm: &VmId,
        vm_type: Option<VmType>,
        input: VmVncproxyInput,
    ) -> Result<Vncproxy> {
        let vm_type = self.vm_type(node, vm, vm_type).await?;
        match vm_type {
            VmType::Lxc => {
                self.post_form_json(
                    &format!("nodes/{node}/lxc/{vm}/vncproxy"),
                    &input.into_lxc(),
                )
                .await
            }
            VmType::Qemu => {
                self.post_form_json(
                    &format!("nodes/{node}/qemu/{vm}/vncproxy"),
                    &input.into_qemu(),
                )
                .await
            }
        }
    }
}

/// Private API
impl ProxmoxApiClient {
    async fn vm_post_status<T, L, Q>(
        &self,
        node: &NodeId,
        vm: &VmId,
        vm_type: Option<VmType>,
        action: &str,
        input: T,
    ) -> Result<String>
    where
        T: VmStatusInput<QemuInput = Q, LxcInput = L>,
        Q: Serialize,
        L: Serialize,
    {
        let vm_type = self.vm_type(node, vm, vm_type).await?;
        match vm_type {
            VmType::Lxc => {
                self.post_form_json(
                    &format!("nodes/{node}/lxc/{vm}/status/{action}"),
                    &input.into_lxc(),
                )
                .await
            }
            VmType::Qemu => {
                self.post_form_json(
                    &format!("nodes/{node}/qemu/{vm}/status/{action}"),
                    &input.into_qemu(),
                )
                .await
            }
        }
    }

    async fn vm_type(
        &self,
        node: &NodeId,
        vm: &VmId,
        maybe_vm_type: Option<VmType>,
    ) -> Result<VmType> {
        match maybe_vm_type {
            Some(vm_type) => Ok(vm_type),
            None => {
                for lxcvm in self.node_lxc(node).await? {
                    if &lxcvm.vmid == vm {
                        return Ok(VmType::Lxc);
                    }
                }
                Ok(VmType::Qemu)
            }
        }
    }

    async fn get_without_params_json<T>(&self, route: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let resp = self.get_without_params(route).await?;
        let status = resp.status();
        resp.json::<Wrapper<T>>()
            .await
            .map_err(Into::into)
            .and_then(|v| self.handle_wrapper(status, v))
    }

    async fn get_json<P, T>(&self, route: &str, params: &P) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let resp = self.get(route, params).await?;
        let status = resp.status();
        resp.json::<Wrapper<T>>()
            .await
            .map_err(Into::into)
            .and_then(|v| self.handle_wrapper(status, v))
    }

    async fn post_form_json<B, T>(&self, route: &str, body: &B) -> Result<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let resp = self.post_form(route, body).await?;
        let status = resp.status();
        resp.json::<Wrapper<T>>()
            .await
            .map_err(Into::into)
            .and_then(|v| self.handle_wrapper(status, v))
    }

    async fn get_without_params(&self, route: &str) -> Result<Response> {
        debug!("GET @ {route}");
        self.do_request(Method::GET, route, |req| req).await
    }

    async fn get<P>(&self, route: &str, params: &P) -> Result<Response>
    where
        P: Serialize + ?Sized,
    {
        debug!("GET @ {route}");
        self.do_request(Method::GET, route, |req| req.query(params))
            .await
    }

    async fn post_form<B>(&self, route: &str, body: &B) -> Result<Response>
    where
        B: Serialize + ?Sized,
    {
        debug!("POST @ {route}");
        self.do_request(Method::POST, route, |req| req.form(body))
            .await
    }

    async fn base_request(&self, method: Method, route: &str) -> Result<RequestBuilder> {
        let auth_header = self.api_access_provider.provide_auth_headers().await?;
        Ok(self
            .client
            .request(method, route)
            .headers(auth_header.extra_headers)
            .header(header::AUTHORIZATION, auth_header.auth_header.as_ref()))
    }

    async fn do_request<F>(
        &self,
        method: Method,
        route: &str,
        modify_request: F,
    ) -> Result<Response>
    where
        F: Fn(RequestBuilder) -> RequestBuilder,
    {
        let mut response = modify_request(self.base_request(method.clone(), route).await?)
            .send()
            .await;

        let status = response
            .as_ref()
            .map(|resp| Some(resp.status()))
            .unwrap_or_else(|err| err.status());

        if status == Some(StatusCode::UNAUTHORIZED) {
            debug!("request failed with 401");
            // Retry with re-auth potentially
            match self.api_access_provider.failed_auth().await {
                DoAfterAuthRetry::Retry => {
                    debug!("access backend indicated retry possible: retrying");
                    response = modify_request(self.base_request(method, route).await?)
                        .send()
                        .await;

                    if response.is_ok() {
                        debug!("retry success");
                    } else {
                        debug!("retry failed");
                        return Err(Error::AuthFailed);
                    }
                }
                DoAfterAuthRetry::Fail => {
                    debug!("retry not possible");
                    return Err(Error::AuthFailed);
                }
            }
        }

        if response.is_ok() {
            self.api_access_provider.auth_success();
        }

        response.map_err(Into::into)
    }

    fn handle_wrapper<T>(&self, status_code: StatusCode, wrapper: Wrapper<T>) -> Result<T> {
        if status_code.is_success() || status_code.is_informational() {
            match wrapper.data {
                None => Err(Error::MissingData),
                Some(v) => Ok(v),
            }
        } else {
            match wrapper.reason {
                None => Err(Error::ApiUnknown(status_code)),
                Some(v) => Err(Error::Api(status_code, v)),
            }
        }
    }
}
