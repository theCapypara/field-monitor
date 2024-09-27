// This is https://gitlab.gnome.org/malureau/rdw/-/blob/master/demo/src/main.rs
// included here for testing.

use std::{
    cell::RefCell,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use adw::prelude::*;
use gio::ApplicationFlags;
use glib::{clone, translate::IntoGlib};
use gtk::{gdk, gio, glib};
use rdw::{DisplayExt, gtk};

use rdw_spice::spice::{self, prelude::*};
use rdw_vnc::gvnc;

fn show_error(app: adw::Application, msg: &str) {
    let mut dialog = adw::MessageDialog::builder()
        .modal(true)
        .heading("Connection error")
        .body(msg);
    if let Some(parent) = app.active_window() {
        dialog = dialog.transient_for(&parent);
    }
    dialog.build().present()
}

async fn show_password_dialog(
    app: adw::Application,
    with_username: bool,
    with_password: bool,
) -> Option<(String, String)> {
    dbg!();
    let grid = gtk::Grid::builder()
        .hexpand(true)
        .vexpand(true)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .row_spacing(6)
        .column_spacing(6)
        .build();
    let mut dialog = adw::MessageDialog::builder()
        .modal(true)
        .extra_child(&grid)
        .default_response("ok")
        .heading("Credentials required");
    if let Some(parent) = app.active_window() {
        dialog = dialog.transient_for(&parent);
    }
    let dialog = dialog.build();
    dialog.add_responses(&[("ok", "Ok")]);

    let username = gtk::Entry::new();
    username.set_activates_default(true);
    if with_username {
        grid.attach(&gtk::Label::new(Some("Username")), 0, 0, 1, 1);
        grid.attach(&username, 1, 0, 1, 1);
    }
    let password = gtk::Entry::new();
    password.set_activates_default(true);
    if with_password {
        grid.attach(&gtk::Label::new(Some("Password")), 0, 1, 1, 1);
        grid.attach(&password, 1, 1, 1, 1);
    }

    let resp = dialog.choose_future().await;
    match resp.as_str() {
        "ok" => Some((username.text().into(), password.text().into())),
        _ => None,
    }
}

fn rdp_display(app: &adw::Application, uri: glib::Uri) -> rdw::Display {
    let rdp = rdw_rdp::Display::new();

    let port = match uri.port() {
        -1 => 3389,
        port => port,
    };
    let host = uri.host().unwrap_or_else(|| "localhost".into());

    rdp.with_settings(|s| {
        s.set_server_port(port as _);
        s.set_server_hostname(Some(host.as_str()))?;
        s.set_remote_fx_codec(true);
        // parse_command_line() sets some extra default stuff, clunky
        s.parse_command_line(&["demo", "/rfx"], true)?;
        Ok(())
    })
    .unwrap();

    rdp.connect_rdp_authenticate(clone!(
        #[weak]
        app,
        #[upgrade_or]
        false,
        move |rdp| {
            glib::MainContext::default().block_on(clone!(
                #[weak]
                app,
                #[upgrade_or]
                false,
                async move {
                    if let Some((username, password)) = show_password_dialog(app, true, true).await
                    {
                        let _ = rdp.with_settings(|s| {
                            s.set_username(Some(&username))?;
                            s.set_password(Some(&password))?;
                            Ok(())
                        });
                        true
                    } else {
                        false
                    }
                }
            ))
        }
    ));

    rdp.connect_notify_local(
        Some("rdp-connected"),
        clone!(
            #[weak]
            app,
            move |rdp, _| {
                let connected = rdp.property::<bool>("rdp-connected");
                log::debug!("Connected: {connected:?}");
                if !connected {
                    log::warn!("Last error: {:?}", rdp.last_error());
                    app.quit();
                }
            }
        ),
    );

    glib::MainContext::default().block_on(clone!(
        #[weak]
        rdp,
        async move {
            if rdp.rdp_connect().await.is_err() {
                log::warn!("Last error: {:?}", rdp.last_error());
                app.quit();
            }
        }
    ));
    rdp.upcast()
}

fn vnc_display(app: &adw::Application, uri: glib::Uri) -> rdw::Display {
    let has_error = Arc::new(AtomicBool::new(false));

    let port = match uri.port() {
        -1 => 5900,
        port => port,
    };
    let host = uri.host().unwrap_or_else(|| "localhost".into());
    let vnc = rdw_vnc::Display::new();
    vnc.connection()
        .open_host(&host, &format!("{}", port))
        .unwrap();

    let has_error2 = has_error.clone();
    vnc.connection().connect_vnc_error(clone!(
        #[weak]
        app,
        move |_, msg| {
            has_error2.store(true, Ordering::Relaxed);
            show_error(app, msg);
        }
    ));

    vnc.connection().connect_vnc_disconnected(clone!(
        #[weak]
        app,
        move |_| {
            if !has_error.load(Ordering::Relaxed) {
                app.quit();
            }
        }
    ));

    vnc.connection().connect_vnc_auth_credential(clone!(
        #[weak]
        app,
        move |conn, va| {
            use gvnc::ConnectionCredential::*;

            let creds: Vec<_> = va
                .iter()
                .map(|v| v.get::<gvnc::ConnectionCredential>().unwrap())
                .collect();
            glib::MainContext::default().spawn_local(clone!(
                #[weak]
                conn,
                async move {
                    if let Some((username, password)) = show_password_dialog(
                        app,
                        creds.contains(&Username),
                        creds.contains(&Password),
                    )
                    .await
                    {
                        if creds.contains(&Username) {
                            conn.set_credential(Username.into_glib(), &username)
                                .unwrap();
                        }
                        if creds.contains(&Password) {
                            conn.set_credential(Password.into_glib(), &password)
                                .unwrap();
                        }
                        if creds.contains(&Clientname) {
                            conn.set_credential(Clientname.into_glib(), "rdw-vnc")
                                .unwrap();
                        }
                    }
                }
            ));
        }
    ));

    vnc.upcast()
}

fn spice_display(app: &adw::Application, uri: glib::Uri) -> rdw::Display {
    let spice = rdw_spice::Display::new();
    let session = spice.session();

    session.set_uri(Some(&uri.to_string()));

    session.connect_channel_new(clone!(
        #[weak]
        app,
        move |_, channel| {
            if let Ok(main) = channel.clone().downcast::<spice::MainChannel>() {
                main.connect_channel_event(clone!(
                    #[weak]
                    app,
                    move |channel, event| {
                        use spice::ChannelEvent::*;
                        if event == ErrorConnect {
                            if let Some(err) = channel.error() {
                                show_error(app, &err.to_string());
                            }
                        }
                    }
                ));
            }
        }
    ));

    session.connect_disconnected(clone!(
        #[weak]
        app,
        move |_| {
            app.quit();
        }
    ));

    session.connect();
    spice.upcast()
}

fn make_display(app: &adw::Application, mut uri: String) -> rdw::Display {
    if glib::Uri::peek_scheme(&uri).is_none() {
        uri = format!("vnc://{}", uri);
    }

    let uri = glib::Uri::parse(&uri, glib::UriFlags::NONE).unwrap();

    match uri.scheme().as_str() {
        "vnc" => vnc_display(app, uri),
        "rdp" => rdp_display(app, uri),
        spice if spice.starts_with("spice") => spice_display(app, uri),
        scheme => panic!("Unhandled scheme {}", scheme),
    }
}

fn main() {
    env_logger::init();
    #[cfg(feature = "bindings")]
    unsafe {
        rdw::setup_logger(log::logger(), log::max_level()).unwrap();
    }

    let app = adw::Application::new(
        Some("org.gnome.rdw.demo"),
        ApplicationFlags::NON_UNIQUE | ApplicationFlags::HANDLES_COMMAND_LINE,
    );
    app.add_main_option(
        "version",
        glib::Char(0),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Show program version",
        None,
    );
    app.add_main_option(
        "debug",
        glib::Char(0),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Enable debugging",
        None,
    );
    app.add_main_option(
        glib::OPTION_REMAINING,
        glib::Char(0),
        glib::OptionFlags::NONE,
        glib::OptionArg::StringArray,
        "URI",
        Some("URI"),
    );
    app.connect_handle_local_options(|_, opt| {
        if opt.lookup_value("version", None).is_some() {
            println!("Version: {}", env!("CARGO_PKG_VERSION"));
            return 0;
        }
        if opt.lookup_value("debug", None).is_some() {
            gvnc::set_debug(true);
            spice::set_debug(true);
        }
        -1
    });

    let display = Arc::new(RefCell::new(None));

    let dpy = display.clone();
    app.connect_command_line(move |app, cl| {
        let uri = cl
            .options_dict()
            .lookup_value(glib::OPTION_REMAINING, None)
            .and_then(|args| args.child_value(0).get::<String>())
            .unwrap_or_else(|| "vnc://localhost".to_string());
        let display = make_display(app, uri);
        dpy.replace(Some(display));
        app.activate();
        -1
    });

    let action_quit = gio::SimpleAction::new("quit", None);
    action_quit.connect_activate(clone!(
        #[weak]
        app,
        move |_, _| {
            app.quit();
        }
    ));
    app.add_action(&action_quit);

    let action_usb = gio::SimpleAction::new("usb", None);
    let dpy = display.clone();
    action_usb.connect_activate(clone!(
        #[weak]
        app,
        move |_, _| {
            let display = match &*dpy.borrow() {
                Some(display) => display.clone(),
                _ => return,
            };

            if let Ok(spice) = display.downcast::<rdw_spice::Display>() {
                let usbredir = match rdw_spice::UsbRedir::build(&spice.session()) {
                    Ok(it) => it,
                    Err(e) => {
                        panic!("Failed to open USB dialog: {}", e);
                    }
                };
                let dialog = gtk::Window::new();
                dialog.set_transient_for(app.active_window().as_ref());
                dialog.set_child(Some(&usbredir));
                dialog.set_visible(true);
            }
        }
    ));
    app.add_action(&action_usb);

    app.connect_activate(move |app| {
        build_ui(app, display.clone());
    });
    app.run();
}

fn build_ui(app: &adw::Application, display: Arc<RefCell<Option<rdw::Display>>>) {
    let ui_src = include_str!("demo.ui");
    let builder = gtk::Builder::new();
    builder
        .add_from_string(ui_src)
        .expect("Couldn't add from string");
    let window: adw::ApplicationWindow = builder.object("window").expect("Couldn't get window");
    window.set_application(Some(app));

    if let Some(display) = &*display.borrow() {
        display.set_vexpand(true);
        display.connect_property_grabbed_notify(clone!(
            #[weak]
            window,
            move |d| {
                let mut title = "rdw demo".to_string();
                if !d.grabbed().is_empty() {
                    title = format!(
                        "{} - {}",
                        title,
                        d.grab_shortcut()
                            .to_label(&gdk::Display::default().unwrap())
                    )
                }
                window.set_title(Some(title.as_str()));
            }
        ));

        let view: gtk::Box = builder.object("view").expect("Couldn't get view");
        view.append(display);
    }

    window.set_visible(true);
}
