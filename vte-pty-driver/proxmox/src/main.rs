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
use anyhow::anyhow;
use async_tungstenite::tokio::client_async_tls_with_connector;
use async_tungstenite::tungstenite;
use async_tungstenite::tungstenite::client::IntoClientRequest;
use async_tungstenite::tungstenite::handshake::client::generate_key;
use async_tungstenite::tungstenite::http::Uri;
use async_tungstenite::tungstenite::{Bytes, Message};
use field_monitor_vte_driver_lib::{PtyClient, args, debug, error, setup_driver};
use futures::prelude::*;
use http::HeaderValue;
use nix::libc;
use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction};
use nix::sys::termios::{SetArg, cfmakeraw, tcgetattr, tcsetattr};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use proxmox_api::{NodeId, ProxmoxApiClient, Termproxy, VmId, VmType, VncwebsocketInput};
use std::error::Error;
use std::mem;
use std::ops::Deref;
use std::os::fd::{AsFd, AsRawFd, RawFd};
use std::pin::Pin;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{Mutex, Notify};
use tokio::time::sleep;
use tokio_util::io::ReaderStream;
pub static CHANGED_WINSIZE_NOTIFY: Notify = Notify::const_new();

extern "C" fn handle_sig(_: libc::c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {}

extern "C" fn handle_sigwinch(_: libc::c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    CHANGED_WINSIZE_NOTIFY.notify_one();
}

mod ioctl {
    use nix::{ioctl_read_bad, libc};

    ioctl_read_bad!(read_term_size, libc::TIOCGWINSZ, libc::winsize);
}

#[tokio::main]
async fn main() -> ! {
    let client = Arc::new(setup_driver().await);

    let result = run_console(&client).await;

    client
        .set_result(
            result
                .as_ref()
                .map(|_| "exited normally")
                .map_err(ToString::to_string),
        )
        .await
        .ok();

    if let Err(err) = &result {
        error!(&client, "failed to run pty driver: {err}");
    }
    debug!(&client, "exiting");
    exit(if result.is_err() { 1 } else { 0 });
}

async fn run_console(client: &Arc<PtyClient>) -> Result<(), anyhow::Error> {
    args!(&client => (
        connection_type,
        root,
        user_tokenid,
        pass_apikey,
        ignore_ssl_errors,
        node_id,
        vm_type,
        vm_id,
        termproxy_str
    ));
    let (vncwebsocket_user, vncwebsocket) = {
        let termproxy: Termproxy = serde_json::from_str(termproxy_str)?;
        (
            termproxy.user,
            VncwebsocketInput {
                port: termproxy.port,
                vncticket: termproxy.ticket,
            },
        )
    };
    let root = Uri::from_str(root)?;
    let node_id = NodeId::from_str(node_id)?;
    let vm = if vm_id.is_empty() {
        None
    } else {
        let vm_id = VmId::from(vm_id.parse::<u64>()?);
        let vm_type = if vm_type == "qemu" {
            VmType::Qemu
        } else {
            VmType::Lxc
        };
        Some((vm_id, vm_type))
    };

    debug!(&client, "running console");

    // Ignore signals, they will be processed via stdin and sent to the remote.
    let sighandler = SigAction::new(
        SigHandler::SigAction(handle_sig),
        SaFlags::SA_SIGINFO,
        SigSet::empty(),
    );

    // Listen for SIGWINCH to transfer size
    let sighandler_sigwinch = SigAction::new(
        SigHandler::SigAction(handle_sigwinch),
        SaFlags::SA_SIGINFO,
        SigSet::empty(),
    );

    // SAFETY: Our signal handler does nothing and (as far as we know) no invalid signal handler
    //         was installed before.
    unsafe {
        sigaction(Signal::SIGQUIT, &sighandler)?;
        sigaction(Signal::SIGTERM, &sighandler)?;
        sigaction(Signal::SIGINT, &sighandler)?;
        sigaction(Signal::SIGHUP, &sighandler)?;
        sigaction(Signal::SIGPIPE, &sighandler)?;
        sigaction(Signal::SIGWINCH, &sighandler_sigwinch)?;
    }

    // Set to raw mode.
    let stdin = std::io::stdin();
    let stdin_fd = stdin.as_fd();
    let mut termios = tcgetattr(stdin_fd)?;
    cfmakeraw(&mut termios);
    tcsetattr(stdin_fd, SetArg::TCSAFLUSH, &termios)?;

    debug!(&client, "setup sigaction");

    let proxmox_client = match connection_type.deref() {
        "apikey" => {
            ProxmoxApiClient::connect_with_apikey(
                &root,
                user_tokenid,
                pass_apikey.into(),
                ignore_ssl_errors == "1",
            )
            .await?
        }
        _ => {
            ProxmoxApiClient::connect_with_ticket(
                &root,
                user_tokenid,
                pass_apikey.into(),
                ignore_ssl_errors == "1",
            )
            .await?
        }
    };

    debug!(&client, "client connected");

    let mut request = match vm {
        None => {
            proxmox_client
                .node_vncwebsocket(&node_id, &vncwebsocket)
                .await?
        }
        Some((vm_id, vm_type)) => {
            proxmox_client
                .vm_vncwebsocket(&node_id, &vm_id, vm_type, &vncwebsocket)
                .await?
        }
    };
    {
        let request_headers = request.headers_mut();
        request_headers.insert("Sec-WebSocket-Version", HeaderValue::from_str("13")?);
        request_headers.insert("Sec-WebSocket-Key", HeaderValue::from_str(&generate_key())?);
    }

    debug!(&client, "request built");

    let request = request.into_client_request()?;

    let domain = domain(&request)?;
    let port = port(&request)?;

    let socket = TcpStream::connect((domain.as_str(), port)).await?;
    let mut connector_builder = SslConnector::builder(SslMethod::tls())?;
    connector_builder.set_verify(if ignore_ssl_errors == "1" {
        SslVerifyMode::NONE
    } else {
        SslVerifyMode::PEER
    });
    let connector = connector_builder.build().configure()?;
    let mut ws = match client_async_tls_with_connector(request, socket, Some(connector)).await {
        Ok((ws, _)) => ws,
        Err(err) => {
            error!(&client, "websocket connection failed: {err:?}");
            return Err(err.into());
        }
    };

    debug!(&client, "ws connected");

    ws.send(Message::text(format!(
        "{}:{}\n",
        vncwebsocket_user, vncwebsocket.vncticket
    )))
    .await?;

    debug!(&client, "hello sent");

    let msg = ws
        .next()
        .await
        .ok_or_else(|| anyhow!("Stream closed after hello"))??;
    match msg {
        Message::Text(s) if &*s == "OK" => {}
        Message::Binary(s) if &*s == b"OK" => {}
        _ => {
            Err(anyhow!("Invalid response to hello"))?;
        }
    }

    let (sink, stream) = ws.split();
    let sink = Arc::new(Mutex::new(Box::pin(sink)));

    debug!(&client, "answer received. starting.");

    select!(
        r = watch_stdin(client.clone(), sink.clone()) => {
            debug!(&client, "error in watch_stdin");
            r
        },
        r = watch_ws(client.clone(), Box::pin(stream)) => {
            debug!(&client, "error in watch_ws");
            r
        },
        r = keep_alive(client.clone(), sink.clone()) => {
            debug!(&client, "error in keep_alive");
            r
        },
        r = watch_term_size(client.clone(), stdin_fd.as_raw_fd(), sink) => {
            debug!(&client, "error in watch_term_size");
            r
        }
    )
}

async fn watch_stdin<S>(
    client: Arc<PtyClient>,
    sink: Arc<Mutex<Pin<Box<S>>>>,
) -> Result<(), anyhow::Error>
where
    S: Sink<Message> + Send + Sync,
    S::Error: Send + Sync + Error + 'static,
{
    debug!(&client, "starting watch_stdin");
    let mut stdin = ReaderStream::new(tokio::io::stdin());
    while let Some(data) = stdin.try_next().await? {
        debug!(&client, "watch_stdin: got data");
        sink.lock()
            .await
            .send(Message::Binary(
                format!("0:{}:", data.len())
                    .into_bytes()
                    .into_iter()
                    .chain(data.into_iter())
                    .collect::<Bytes>(),
            ))
            .await?;
        debug!(&client, "watch_stdin: sent data");
    }
    Ok(())
}

async fn keep_alive<S>(
    client: Arc<PtyClient>,
    sink: Arc<Mutex<Pin<Box<S>>>>,
) -> Result<(), anyhow::Error>
where
    S: Sink<Message> + Send + Sync,
    S::Error: Send + Sync + Error + 'static,
{
    loop {
        sleep(Duration::from_secs(30)).await;
        debug!(&client, "keep alive");
        sink.lock().await.send(Message::Text("2".into())).await?;
    }
}

async fn watch_term_size<S>(
    client: Arc<PtyClient>,
    stdin_fd: RawFd,
    sink: Arc<Mutex<Pin<Box<S>>>>,
) -> Result<(), anyhow::Error>
where
    S: Sink<Message> + Send + Sync,
    S::Error: Send + Sync + Error + 'static,
{
    loop {
        let (width, height) = unsafe {
            let mut size: libc::winsize = mem::zeroed();
            ioctl::read_term_size(stdin_fd, &mut size)?;
            (size.ws_col, size.ws_row)
        };
        debug!(&client, "watch_term_size: {width}x{height}");
        sink.lock()
            .await
            .send(Message::Text(format!("1:{}:{}:", width, height).into()))
            .await?;
        CHANGED_WINSIZE_NOTIFY.notified().await;
    }
}

async fn watch_ws<S>(client: Arc<PtyClient>, mut stream: Pin<Box<S>>) -> Result<(), anyhow::Error>
where
    S: Stream<Item = tungstenite::Result<Message>>,
{
    debug!(&client, "starting watch_ws");
    let mut stdout = tokio::io::stdout();
    while let Some(msg) = stream.try_next().await? {
        debug!(&client, "watch_ws: got msg");
        let data = match msg {
            Message::Text(data) => data.into(),
            Message::Binary(data) => data,
            _ => continue,
        };

        stdout.write_all(&data).await?;
        stdout.flush().await?;
        debug!(&client, "watch_ws: sent data");
    }
    Ok(())
}

// These utility functions are from async_tungstenite.

/// Get a domain from an URL.
#[inline]
pub(crate) fn domain(
    request: &tungstenite::handshake::client::Request,
) -> Result<String, tungstenite::Error> {
    request
        .uri()
        .host()
        .map(|host| {
            // If host is an IPv6 address, it might be surrounded by brackets. These brackets are
            // *not* part of a valid IP, so they must be stripped out.
            //
            // The URI from the request is guaranteed to be valid, so we don't need a separate
            // check for the closing bracket.
            let host = if host.starts_with('[') {
                &host[1..host.len() - 1]
            } else {
                host
            };

            host.to_owned()
        })
        .ok_or(tungstenite::Error::Url(
            tungstenite::error::UrlError::NoHostName,
        ))
}

/// Get the port from an URL.
#[inline]
pub(crate) fn port(
    request: &tungstenite::handshake::client::Request,
) -> Result<u16, tungstenite::Error> {
    request
        .uri()
        .port_u16()
        .or_else(|| match request.uri().scheme_str() {
            Some("wss") => Some(443),
            Some("ws") => Some(80),
            _ => None,
        })
        .ok_or(tungstenite::Error::Url(
            tungstenite::error::UrlError::UnsupportedUrlScheme,
        ))
}
