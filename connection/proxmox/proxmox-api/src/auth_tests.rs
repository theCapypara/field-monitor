use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

const CHALLENGE: &str = r#"{"data":{"ticket":"PVE:!tfa!%7B%22totp%22%3Atrue%7D:123::signature","CSRFPreventionToken":"csrf","NeedTFA":1}}"#;
const SESSION: &str =
    r#"{"data":{"ticket":"PVE:user@pam:123::signature","CSRFPreventionToken":"csrf"}}"#;

// A real HTTP boundary checks form encoding, ordering and the absence of challenge cookies.
fn server(script: Vec<(u16, &'static str)>) -> (Uri, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let root = format!("http://{}/api2/json/", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body) in script {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "expected another HTTP request");
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).unwrap();
                assert_ne!(n, 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            requests.push(String::from_utf8(bytes).unwrap());
            write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        requests
    });
    (root, handle)
}

async fn client(root: &Uri) -> ProxmoxApiClient {
    ProxmoxApiClient::connect_with_ticket(root, "user@pam", "password".into(), None, false)
        .await
        .unwrap()
}

#[test]
fn challenge_detection_and_session_import() {
    for flag in ["null", "false", "0"] {
        let json = format!(
            r#"{{"ticket":"PVE:user@pam:123::signature","CSRFPreventionToken":"csrf","NeedTFA":{flag}}}"#
        );
        let ticket: Ticket = serde_json::from_str(&json).unwrap();
        assert!(!ticket.is_challenge());
        ticket.validate_session().unwrap();
    }
    let ticket: Wrapper<Ticket> = serde_json::from_str(CHALLENGE).unwrap();
    assert!(matches!(
        ticket.data.unwrap().validate_session(),
        Err(Error::TfaRequired)
    ));
    let root: Uri = "https://pve.example/api2/json/".parse().unwrap();
    let json = serde_json::to_string(
        &serde_json::from_str::<Wrapper<Ticket>>(CHALLENGE)
            .unwrap()
            .data
            .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        ProxmoxApiClient::connect_with_session(&root, "user@pam", json.into(), false),
        Err(Error::TfaRequired)
    ));
}

#[tokio::test]
async fn password_only_login_and_session_reuse() {
    let (root, server) = server(vec![
        (200, SESSION),
        (200, r#"{"data":[]}"#),
        (200, r#"{"data":[]}"#),
    ]);
    let client = client(&root).await;
    client.nodes().await.unwrap();
    client.nodes().await.unwrap();
    let requests = server.join().unwrap();
    assert!(requests[0].contains("password=password"));
    assert!(requests[1].contains("PVEAuthCookie=PVE:user@pam:123::signature"));
    assert!(requests[2].starts_with("GET"));
}

#[tokio::test]
async fn totp_challenge_is_retained_and_session_transfers_to_helper() {
    let (root, server) = server(vec![
        (200, CHALLENGE),
        (200, SESSION),
        (200, r#"{"data":[]}"#),
    ]);
    let client = client(&root).await;
    assert!(matches!(
        client.authenticate().await,
        Err(Error::TfaRequired)
    ));
    // Repeated loads must not restart the password exchange.
    assert!(matches!(
        client.authenticate().await,
        Err(Error::TfaRequired)
    ));
    client.submit_totp(" 012345 ".into()).await.unwrap();
    let session = client.session_ticket().await.unwrap().unwrap();
    let helper = ProxmoxApiClient::connect_with_session(&root, "user@pam", session, false).unwrap();
    helper.nodes().await.unwrap();
    let requests = server.join().unwrap();
    assert!(requests[1].contains("password=totp%3A012345"));
    assert!(requests[1].contains("tfa-challenge=PVE%3A%21tfa%21"));
    assert!(requests[2].starts_with("GET"));
    assert!(!requests[2].contains("tfa!"));
}

#[tokio::test]
async fn authentication_status_is_checked_before_json() {
    for status in [401, 403, 500] {
        let (root, server) = server(vec![(status, "not JSON")]);
        let result = client(&root).await.authenticate().await;
        if status == 500 {
            assert!(matches!(result, Err(Error::ApiUnknown(_))));
        } else {
            assert!(matches!(result, Err(Error::AuthFailed)));
        }
        server.join().unwrap();
    }
}

#[tokio::test]
async fn incorrect_code_can_be_retried_with_a_fresh_challenge() {
    let (root, server) = server(vec![
        (200, CHALLENGE),
        (401, ""),
        (200, CHALLENGE),
        (200, SESSION),
    ]);
    let client = client(&root).await;
    assert!(matches!(
        client.authenticate().await,
        Err(Error::TfaRequired)
    ));
    assert!(matches!(
        client.submit_totp("000000".into()).await,
        Err(Error::TfaRejected)
    ));
    client.submit_totp("123456".into()).await.unwrap();
    let requests = server.join().unwrap();
    assert!(requests[2].contains("password=password"));
    assert!(requests[3].contains("password=totp%3A123456"));
}

#[tokio::test]
async fn partial_ticket_after_totp_is_never_used_as_a_session() {
    let (root, server) = server(vec![(200, CHALLENGE), (200, CHALLENGE)]);
    let client = client(&root).await;
    assert!(matches!(
        client.authenticate().await,
        Err(Error::TfaRequired)
    ));
    assert!(matches!(
        client.submit_totp("123456".into()).await,
        Err(Error::TfaRequired)
    ));
    assert!(matches!(
        client.session_ticket().await,
        Err(Error::TfaRequired)
    ));
    server.join().unwrap();
}

#[tokio::test]
async fn concurrent_requests_share_one_login() {
    let (root, server) = server(vec![
        (200, CHALLENGE),
        (200, SESSION),
        (200, r#"{"data":[]}"#),
        (200, r#"{"data":[]}"#),
    ]);
    let client = ProxmoxApiClient::connect_with_ticket(
        &root,
        "user@pam",
        "password".into(),
        Some("123456".into()),
        false,
    )
    .await
    .unwrap();
    let (a, b) = futures::join!(client.nodes(), client.nodes());
    a.unwrap();
    b.unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.iter().filter(|r| r.starts_with("POST")).count(), 2);
}

#[tokio::test]
async fn renewal_uses_ticket_and_retry_401_stays_an_auth_error() {
    let (root, server) = server(vec![
        (200, SESSION),
        (200, r#"{"data":[]}"#),
        (401, ""),
        (200, SESSION),
        (401, ""),
    ]);
    let client = client(&root).await;
    client.nodes().await.unwrap();
    assert!(matches!(client.nodes().await, Err(Error::AuthFailed)));
    let requests = server.join().unwrap();
    assert!(requests[3].contains("password=PVE%3Auser%40pam%3A123%3A%3Asignature"));
    assert!(!requests[3].contains("password=password"));
}

#[tokio::test]
async fn locally_invalid_codes_do_not_reach_the_server() {
    let root: Uri = "http://127.0.0.1:1/api2/json/".parse().unwrap();
    let client = client(&root).await;
    for code in ["", "12345", "123456789", "abcdef", "123 456"] {
        assert!(matches!(
            client.submit_totp(code.into()).await,
            Err(Error::InvalidTotp)
        ));
    }
}

#[tokio::test]
async fn webauthn_only_account_does_not_submit_a_totp() {
    let (root, server) = server(vec![(
        200,
        r#"{"data":{"ticket":"PVE:!tfa!%7B%22webauthn%22%3A%7B%7D%7D:123::signature","CSRFPreventionToken":"csrf","NeedTFA":1}}"#,
    )]);
    assert!(matches!(
        client(&root).await.submit_totp("123456".into()).await,
        Err(Error::UnsupportedTfa)
    ));
    assert_eq!(server.join().unwrap().len(), 1);
}

#[tokio::test]
async fn aging_session_is_renewed_before_use_without_an_otp() {
    let (root, server) = server(vec![(200, SESSION)]);
    let ticket = serde_json::from_str::<Wrapper<Ticket>>(SESSION)
        .unwrap()
        .data
        .unwrap();
    let provider = TicketProvider {
        client: Client::new(&root, false).unwrap(),
        user: "user@pam".into(),
        password: "password".into(),
        tfa_response: Default::default(),
        just_reauth: Default::default(),
        current_ticket: Arc::new(Mutex::new(Some(ticket))),
        pending_challenge: Default::default(),
        auth_lock: Default::default(),
        renewed_at: Mutex::new(Some(Instant::now() - Duration::from_secs(91 * 60))),
    };
    provider.provide_auth_headers().await.unwrap();
    let requests = server.join().unwrap();
    assert!(requests[0].contains("password=PVE%3Auser%40pam%3A123%3A%3Asignature"));
}

#[tokio::test]
async fn late_unauthorized_response_preserves_replacement_session() {
    let root: Uri = "http://127.0.0.1:1/api2/json/".parse().unwrap();
    let ticket = serde_json::from_str::<Wrapper<Ticket>>(SESSION)
        .unwrap()
        .data
        .unwrap();
    let provider = TicketProvider {
        client: Client::new(&root, false).unwrap(),
        user: "user@pam".into(),
        password: "password".into(),
        tfa_response: Default::default(),
        just_reauth: AtomicBool::new(true),
        current_ticket: Arc::new(Mutex::new(Some(ticket.clone()))),
        pending_challenge: Default::default(),
        auth_lock: Default::default(),
        renewed_at: Mutex::new(Some(Instant::now())),
    };
    let stale = "PVEAuthCookie=PVE:user@pam:old::signature";
    assert!(matches!(
        provider.failed_auth(stale).await,
        DoAfterAuthRetry::Retry
    ));
    provider.invalidate_session(stale).await;
    assert!(provider.current_ticket.lock().await.as_ref() == Some(&ticket));
    assert!(provider.renewed_at.lock().await.is_some());

    provider
        .invalidate_session(&format!("PVEAuthCookie={}", ticket.ticket))
        .await;
    assert!(provider.current_ticket.lock().await.is_none());
    assert!(provider.renewed_at.lock().await.is_none());
}
