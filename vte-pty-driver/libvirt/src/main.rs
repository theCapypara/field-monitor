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
use std::io;
use std::io::{stdin, stdout, Read, Write};
use std::mem::MaybeUninit;
use std::os::fd::AsFd;
use std::process::exit;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use nix::libc;
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::sys::termios::{cfmakeraw, tcgetattr, tcsetattr, SetArg};
use parking_lot::Mutex;
use ringbuf::storage::Heap;
use ringbuf::traits::*;
use ringbuf::{CachingCons, CachingProd, HeapRb, SharedRb};
use tokio::runtime::Handle;
use tokio::select;
use virt::connect::Connect;
use virt::domain::Domain;
use virt::stream::Stream;
use virt::sys::{
    virEventAddTimeout, virEventRegisterDefaultImpl, virEventRunDefaultImpl,
    virStreamEventAddCallback, virStreamFlags, virStreamPtr, virStreamRecv, virStreamSend,
    VIR_DOMAIN_CONSOLE_FORCE, VIR_STREAM_EVENT_READABLE, VIR_STREAM_EVENT_WRITABLE,
    VIR_STREAM_NONBLOCK,
};

use field_monitor_vte_driver_lib::{args, debug, debug_sync, error, setup_driver, PtyClient};

struct ConsoleContext {
    #[allow(unused)]
    pty_client: Arc<PtyClient>,
    stream: Stream,
    term_eof: AtomicBool,
    stream_eof: AtomicBool,
    watch_stream_result: Mutex<Option<Result<(), anyhow::Error>>>,
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
    args!(&client => (qemu_ui, domid));

    debug!(&client, "running console");

    // Ignore signals, they will be processed via stdin and sent to the remote.
    let sighandler = SigAction::new(
        SigHandler::SigAction(handle_sig),
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
    }

    // Set to raw mode.
    {
        let stdin = stdin();
        let stdin_fd = stdin.as_fd();
        let mut termios = tcgetattr(stdin_fd)?;
        cfmakeraw(&mut termios);
        tcsetattr(stdin_fd, SetArg::TCSAFLUSH, &termios)?;
    }

    debug!(&client, "setup sigaction");

    // SAFETY: This function is safe to call.
    let ret = unsafe { virEventRegisterDefaultImpl() };
    if ret == -1 {
        debug!(&client, "failed to register event loop");
        return Err(virt::error::Error::last_error().into());
    }
    debug!(&client, "registered libvirt event loop");

    let connect = Connect::open(Some(qemu_ui))?;
    let domain = Domain::lookup_by_uuid_string(&connect, domid)?;

    let st = Stream::new(&connect, VIR_STREAM_NONBLOCK)?;
    debug!(&client, "opened stream");

    domain.open_console(None, &st, VIR_DOMAIN_CONSOLE_FORCE)?;
    debug!(&client, "opened console");

    debug!(&client, "established domain connection");

    let rt = Handle::current();

    let term_bytes = HeapRb::<u8>::new(12288);
    let (term_prod, term_cons) = term_bytes.split();
    let stream_bytes = HeapRb::<u8>::new(12288);
    let (stream_prod, stream_cons) = stream_bytes.split();

    let context = Arc::new(ConsoleContext {
        pty_client: client.clone(),
        stream: st,
        term_eof: Default::default(),
        stream_eof: Default::default(),
        watch_stream_result: Default::default(),
    });

    let context_cln1 = context.clone();
    let context_cln2 = context.clone();
    let context_cln3 = context.clone();
    let watch_stdin = rt.spawn_blocking(move || watch_stdin(context_cln1, term_prod));
    let watch_stdout = rt.spawn_blocking(move || watch_stdout(context_cln2, stream_cons));
    let watch_stream =
        rt.spawn_blocking(move || watch_stream(context_cln3, stream_prod, term_cons));

    select!(
        r = watch_stdin => {
            debug!(&client, "error in watch_stdin");
            r
        },
        r = watch_stdout => {
            debug!(&client, "error in watch_stdout");
            r
        },
        r = watch_stream => {
            debug!(&client, "error in watch_stream");
            r
        }
    )
    .unwrap_or_else(|e| Err(e.into()))
}

fn watch_stdin(
    context: Arc<ConsoleContext>,
    mut term_prod: CachingProd<Arc<SharedRb<Heap<u8>>>>,
) -> Result<(), anyhow::Error> {
    debug_sync!(&context.pty_client, "starting watch_stdin");
    let mut stdin = stdin().lock();
    loop {
        let result = term_prod.read_from(&mut stdin, None);
        debug_sync!(&context.pty_client, "stdin read res: {:?}", &result);

        if term_prod.occupied_len() > 0 {
            context
                .stream
                .event_update_callback(VIR_STREAM_EVENT_READABLE | VIR_STREAM_EVENT_WRITABLE)
                .ok();
        }

        match result {
            None => {
                debug_sync!(
                    &context.pty_client,
                    "watch_stdin: ringbuffer read returned None"
                );
                sleep(Duration::from_millis(5));
                continue;
            }
            Some(Ok(0)) => {
                debug_sync!(&context.pty_client, "watch_stdin: EOF");
                context.term_eof.store(true, Ordering::Release);
                return Ok(());
            }
            Some(Ok(_)) => {}
            Some(Err(e)) => {
                debug_sync!(&context.pty_client, "watch_stdin: err");
                context.term_eof.store(true, Ordering::Release);
                return Err(e.into());
            }
        }
    }
}

fn watch_stdout(
    context: Arc<ConsoleContext>,
    mut stream_cons: CachingCons<Arc<SharedRb<Heap<u8>>>>,
) -> Result<(), anyhow::Error> {
    debug_sync!(&context.pty_client, "starting watch_stdout");
    let mut stdout = stdout().lock();
    loop {
        if stream_cons.is_empty() {
            if context.stream_eof.load(Ordering::Acquire) {
                return Ok(());
            }
            sleep(Duration::from_millis(5));
            continue;
        }
        let (a, b) = stream_cons.as_slices();
        let mut sum = 0;
        let mut result = stdout.write(a);
        debug_sync!(&context.pty_client, "stdout write res1: {:?}", &result);
        if let Ok(v) = &result {
            sum += *v;
            if !b.is_empty() {
                result = stdout.write(b);
                debug_sync!(&context.pty_client, "stdout write res2: {:?}", &result);
                if let Ok(v) = &result {
                    sum += *v;
                }
            }
        }
        stdout.flush().ok();

        match result {
            Ok(0) => {
                debug_sync!(&context.pty_client, "watch_stdout: eof");
                return Ok(());
            }
            Ok(_) => {
                stream_cons.skip(sum);
            }
            Err(e) => {
                debug_sync!(&context.pty_client, "watch_stdout: err");
                return Err(e.into());
            }
        }
    }
}

struct StreamWatchContext {
    context: Arc<ConsoleContext>,
    stream_prod: CachingProd<Arc<SharedRb<Heap<u8>>>>,
    term_cons: CachingCons<Arc<SharedRb<Heap<u8>>>>,
}

extern "C" fn stream_event_callback(
    _: virStreamPtr,
    events: libc::c_int,
    opaque: *mut libc::c_void,
) {
    let events = events as virStreamFlags;
    // SAFETY: We know we passed in a valid lived object here.
    let stream_ctx = unsafe {
        let mu = &mut *(opaque as *mut MaybeUninit<StreamWatchContext>);
        mu.assume_init_mut()
    };
    let context = &mut stream_ctx.context;
    let stream_prod = &mut stream_ctx.stream_prod;
    let term_cons = &mut stream_ctx.term_cons;

    let mut stream = context.stream.clone();
    debug_sync!(&context.pty_client, "got stream callback event");

    fn set_result(context: &ConsoleContext, result: Result<(), anyhow::Error>) {
        *context.watch_stream_result.lock() = Some(result);
    }

    if (events & VIR_STREAM_EVENT_READABLE) > 0 {
        let result = stream_prod.read_from(&mut StreamReadAdapter(&mut stream), None);
        debug_sync!(&context.pty_client, "stream read res: {:?}", &result);

        match result {
            None => {
                debug_sync!(&context.pty_client, "ringbuffer read returned None");
            }
            Some(Ok(0)) => {
                debug_sync!(&context.pty_client, "watch_stream: read eof");
                set_result(context, Ok(()));
                stream.finish().ok();
                return;
            }
            Some(Ok(_)) => {}
            Some(Err(e)) => {
                debug_sync!(&context.pty_client, "watch_stream: read err");
                set_result(context, Err(e.into()));
                stream.finish().ok();
                return;
            }
        }
    }

    if (events & VIR_STREAM_EVENT_WRITABLE) > 0 {
        if term_cons.is_empty() {
            if context.term_eof.load(Ordering::Acquire) {
                set_result(context, Ok(()));
                stream.finish().ok();
            }
            return;
        }
        let (a, b) = term_cons.as_slices();
        let mut sum = 0;
        let mut result = send(&stream, a);
        debug_sync!(&context.pty_client, "stream write res1: {:?}", &result);
        if let Ok(Some(v)) = &result {
            sum += v;
            if !b.is_empty() {
                result = send(&stream, b);
                debug_sync!(&context.pty_client, "stream write res2: {:?}", &result);
                if let Ok(Some(v)) = &result {
                    sum += v;
                }
            }
        }

        match result {
            Ok(None) => {
                term_cons.skip(sum);
            }
            Ok(Some(v)) => {
                term_cons.skip(sum);
                if v == 0 {
                    debug_sync!(&context.pty_client, "watch_stream: write eof");
                    set_result(context, Ok(()));
                    stream.finish().ok();
                }
            }
            Err(e) => {
                debug_sync!(&context.pty_client, "watch_stream: write err");
                set_result(context, Err(e.into()));
                stream.finish().ok();
            }
        }
    }

    if term_cons.occupied_len() == 0 {
        context
            .stream
            .event_update_callback(VIR_STREAM_EVENT_READABLE)
            .ok();
    }
}

extern "C" fn stream_event_free(opaque: *mut libc::c_void) {
    // SAFETY: We know we passed in a valid object here.
    unsafe {
        let mu = &mut *(opaque as *mut MaybeUninit<StreamWatchContext>);
        debug_sync!(
            &mu.assume_init_ref().context.pty_client,
            "stream_event_free"
        );
        mu.assume_init_drop();
    };
}

fn watch_stream(
    context: Arc<ConsoleContext>,
    stream_prod: CachingProd<Arc<SharedRb<Heap<u8>>>>,
    term_cons: CachingCons<Arc<SharedRb<Heap<u8>>>>,
) -> Result<(), anyhow::Error> {
    debug_sync!(&context.pty_client, "starting watch_stream");

    let stream_watch_ctx = StreamWatchContext {
        context,
        stream_prod,
        term_cons,
    };

    // the bindings create an un-callable lifetime condition here, also we can't easily pass state
    // anyway to it as-is, so we do it manually.
    let context = stream_watch_ctx.context.clone();
    let context_pass = Box::leak(Box::new(MaybeUninit::new(stream_watch_ctx)));

    // SAFETY: This is safe to call as long as all pointers are valid and live long enough,
    //         which they do.
    let ret = unsafe {
        let ptr = context_pass as *mut _ as *mut _;
        virStreamEventAddCallback(
            context.stream.as_ptr(),
            VIR_STREAM_EVENT_READABLE as libc::c_int,
            Some(stream_event_callback),
            ptr,
            Some(stream_event_free),
        )
    };
    if ret == -1 {
        debug_sync!(&context.pty_client, "failed to register callback");
        Err(virt::error::Error::last_error().into())
    } else {
        // Add a timeout to the event loop to wake up every so often. Then run the loop.

        // SAFETY: All parameters and function pointers are valid and correct
        let ret =
            unsafe { virEventAddTimeout(500, Some(null_callback), null_mut(), Some(null_free)) };
        if ret == -1 {
            debug_sync!(&context.pty_client, "failed registering timeout");
            return Err(virt::error::Error::last_error().into());
        }

        loop {
            // SAFETY: This is safe to call
            let ret = unsafe { virEventRunDefaultImpl() };
            if ret == -1 {
                debug_sync!(&context.pty_client, "failed event loop iteration");
                return Err(virt::error::Error::last_error().into());
            }
            let mut result_lock = context.watch_stream_result.lock();
            if result_lock.is_some() {
                context.stream_eof.store(true, Ordering::Release);
                return result_lock.take().unwrap();
            }
        }
    }
}

extern "C" fn handle_sig(_: libc::c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {}
extern "C" fn null_callback(_: libc::c_int, _: *mut libc::c_void) {}
extern "C" fn null_free(_: *mut libc::c_void) {}

// recv in the bindings has a bug: -2 is not a normal error condition.
fn recv(stream: &Stream, buf: &mut [u8]) -> Result<Option<usize>, virt::error::Error> {
    let ret = unsafe {
        virStreamRecv(
            stream.as_ptr(),
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
        )
    };
    if ret == -2 {
        Ok(None)
    } else {
        Ok(Some(
            usize::try_from(ret).map_err(|_| virt::error::Error::last_error())?,
        ))
    }
}

// send in the bindings has a bug: -2 is not a normal error condition.
fn send(stream: &Stream, data: &[u8]) -> Result<Option<usize>, virt::error::Error> {
    let ret = unsafe {
        virStreamSend(
            stream.as_ptr(),
            data.as_ptr() as *mut libc::c_char,
            data.len(),
        )
    };
    if ret == -2 {
        Ok(None)
    } else {
        Ok(Some(
            usize::try_from(ret).map_err(|_| virt::error::Error::last_error())?,
        ))
    }
}

struct StreamReadAdapter<'a>(&'a mut Stream);

impl<'a> Read for StreamReadAdapter<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let res = recv(self.0, buf);
            match res {
                Ok(None) => sleep(Duration::from_millis(5)),
                Ok(Some(v)) => return Ok(v),
                Err(err) => return Err(io::Error::new(io::ErrorKind::Other, Box::new(err))),
            }
        }
    }
}
