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
use http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use http::header::AUTHORIZATION;
use reqwest::{Body, ClientBuilder, RequestBuilder, Response};
use secure_string::SecureString;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
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

#[derive(Deserialize, Debug, Clone)]
struct Ticket {
    ticket: String,
    #[serde(rename = "CSRFPreventionToken")]
    csrf_prevention_token: String,
}

#[derive(Deserialize, Debug, Clone)]
struct TicketOuter {
    data: Ticket,
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
                .json::<TicketOuter>()
                .await?
                .data,
        );
        Ok(())
    }
}

struct ApikeyProvider {
    client: Client,
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
                auth_header: format!("PVEAPIToken={}!{}", self.tokenid, self.apikey.unsecure())
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
            if let Some(ticket) = &*current_ticket.lock().await {
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
                    Err(_) => DoAfterAuthRetry::Fail,
                }
            }
        })
    }
}

pub struct ProxmoxApiClient {
    client: Client,
    api_access_provider: Box<dyn ApiAccessProvider + Send + Sync>,
}

impl ProxmoxApiClient {
    pub async fn connect_with_apikey(
        root: &Uri,
        tokenid: &str,
        apikey: SecureString,
        ignore_ssl_errors: bool,
    ) -> Result<Self> {
        let client = Client::new(root, ignore_ssl_errors)?;
        Ok(Self {
            client: client.clone(),
            api_access_provider: Box::new(ApikeyProvider {
                client,
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

    async fn get_json<P, T>(&self, route: &str, params: &P) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.get(route, params)
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    async fn get<P>(&self, route: &str, params: &P) -> Result<Response>
    where
        P: Serialize + ?Sized,
    {
        self.do_request(Method::GET, route, |req| req.query(params))
            .await
    }

    async fn post_json<B, T>(&self, route: &str, body: B) -> Result<T>
    where
        B: Into<Body> + Clone,
        T: DeserializeOwned,
    {
        self.post(route, body)
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    async fn post<B>(&self, route: &str, body: B) -> Result<Response>
    where
        B: Into<Body> + Clone,
    {
        self.do_request(Method::POST, route, |req| req.body(body.clone()))
            .await
    }

    async fn base_request(&self, method: Method, route: &str) -> Result<RequestBuilder> {
        let auth_header = self.api_access_provider.provide_auth_headers().await?;
        Ok(self
            .client
            .request(method, route)
            .headers(auth_header.extra_headers)
            .header(AUTHORIZATION, auth_header.auth_header.as_ref()))
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
            // Retry with re-auth potentially
            match self.api_access_provider.failed_auth().await {
                DoAfterAuthRetry::Retry => {
                    response = modify_request(self.base_request(method, route).await?)
                        .send()
                        .await;

                    if response.is_ok() {
                        self.api_access_provider.auth_success();
                    }
                }
                DoAfterAuthRetry::Fail => {
                    // fall below
                }
            }
        }

        response.map_err(Into::into)
    }
}
