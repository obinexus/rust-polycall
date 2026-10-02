//! Integration tests against the REAL installed libpolycall (binding ABI v1)
//! and the real `polycall` CLI (runtime + C peer node). No mocks.
//!
//! Requirements: the library linked by build.rs, and the CLI on PATH or in
//! `POLYCALL_CLI`. A missing CLI fails the tests that need it (it is never
//! treated as a pass).

use rust_polycall::{self as pc, status, sys, Peer, WAIT_FOREVER};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const TOKEN: &str = "rust-polycall-test-token-5d1e";

fn cli() -> String {
    std::env::var("POLYCALL_CLI").unwrap_or_else(|_| "polycall".to_owned())
}

fn tmp_path(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "rust-polycall-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn write_file(name: &str, text: &str) -> String {
    let path = tmp_path(name);
    std::fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}

/// A CLI child process (runtime or peer node), killed on drop.
struct Proc {
    child: Child,
    endpoint: String,
}

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_with_endpoint(args: &[&str]) -> Proc {
    let file = tmp_path("endpoint");
    let mut cmd = Command::new(cli());
    cmd.args(args)
        .arg("--endpoint-file")
        .arg(&file)
        .env("POLYCALL_TELEMETRY", "off")
        .env("POLYCALL_DEV_TOKEN", TOKEN)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("cannot run the polycall CLI '{}': {e}", cli()));
    // owned by Proc from here on, so every exit path kills and reaps it
    let mut proc = Proc {
        child,
        endpoint: String::new(),
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(text) = std::fs::read_to_string(&file) {
            let ep = text.trim().to_owned();
            if !ep.is_empty() {
                proc.endpoint = ep;
                return proc;
            }
        }
        assert!(
            Instant::now() < deadline,
            "{args:?} did not publish an endpoint"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn start_runtime() -> Proc {
    spawn_with_endpoint(&["start", "--endpoint", "127.0.0.1:0"])
}

fn start_c_peer(node_id: &str) -> Proc {
    spawn_with_endpoint(&[
        "peer",
        "serve",
        "--node-id",
        node_id,
        "--endpoint",
        "127.0.0.1:0",
    ])
}

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn node(id: &str) -> Peer {
    Peer::open(id, Some("127.0.0.1:0"), Some(TOKEN)).expect("open node")
}

fn code<T: std::fmt::Debug>(r: pc::Result<T>) -> i32 {
    r.expect_err("expected an error").code
}

// ---------------------------------------------------------------- library

#[test]
fn version_and_abi_check() {
    pc::check_abi().expect("ABI v1");
    assert_eq!(pc::abi_version(), 1);
    let v = pc::version().unwrap();
    assert!(v.starts_with("1."), "version {v}");
    assert!(!pc::library_version().unwrap().is_empty());
    assert!(pc::strerror(status::TIMEOUT).starts_with("POLYCALL_E_TIMEOUT"));
    assert!(pc::strerror(-999).starts_with("POLYCALL_E_UNKNOWN"));
    assert!(pc::strerror(0).starts_with("POLYCALL_OK"));
}

// ---------------------------------------------------------- configuration

#[test]
fn run_config_valid_missing_invalid_strict_tls() {
    let repo_rc = concat!(env!("CARGO_MANIFEST_DIR"), "/rust-polycallrc");
    pc::run_config(Some(repo_rc), true).expect("shipped rust-polycallrc is valid for running");
    pc::run_config(Some(repo_rc), false).expect("validate");

    let missing = tmp_path("nope-polycallrc");
    let e = pc::run_config(Some(missing.to_str().unwrap()), true).unwrap_err();
    assert_eq!(e.code, status::NOT_FOUND, "{e}");
    assert!(e.name.starts_with("POLYCALL_E_NOT_FOUND"));
    assert!(e.detail.contains("nope-polycallrc"), "detail: {}", e.detail);

    let bad = write_file("bad-polycallrc", "no equals sign on this line\n");
    let e = pc::run_config(Some(&bad), false).unwrap_err();
    assert_eq!(e.code, status::CONFIG, "{e}");
    assert!(
        !e.detail.is_empty(),
        "E_CONFIG must carry the core's detail"
    );
    let bad_value = write_file("value-polycallrc", "max_connections=many\n");
    assert_eq!(
        code(pc::run_config(Some(&bad_value), false)),
        status::CONFIG
    );

    let unknown = write_file("unknown-polycallrc", "log_level=info\nmystery_key=1\n");
    pc::run_config(Some(&unknown), false).expect("unknown keys are warnings when validating");
    let e = pc::run_config(Some(&unknown), true).unwrap_err();
    assert_eq!(e.code, status::CONFIG, "strict: {e}");

    let tls = write_file(
        "tls-polycallrc",
        "tls_enabled=true\ncert_file=/x/cert.pem\nkey_file=/x/key.pem\n",
    );
    pc::run_config(Some(&tls), false).expect("tls file validates");
    let e = pc::run_config(Some(&tls), true).unwrap_err();
    assert_eq!(e.code, status::UNSUPPORTED, "{e}");

    assert_eq!(code(pc::run_config(None, true)), status::INVALID_ARGUMENT);
    assert_eq!(
        code(pc::run_config(Some(""), true)),
        status::INVALID_ARGUMENT
    );
    let e = pc::run_config(Some("a\0b"), true).unwrap_err();
    assert_eq!(e.code, status::INVALID_ARGUMENT);
    assert!(e.detail.contains("NUL"));
}

#[test]
fn describe_returns_json() {
    let rc = write_file(
        "describe-polycallrc",
        "log_level=debug\nmax_connections=7\n",
    );
    let json = pc::describe(Some(&rc)).unwrap();
    assert!(json.trim_start().starts_with('{'), "{json}");
    assert!(json.contains("max_connections"), "{json}");
    assert_eq!(
        code(pc::describe(Some(tmp_path("missing").to_str().unwrap()))),
        status::NOT_FOUND
    );
}

// --------------------------------------------------------------------- RPC

#[test]
fn call_against_polycall_start() {
    let rt = start_runtime();
    let ep = rt.endpoint.as_str();

    let out = pc::call(
        ep,
        "inventory",
        "get",
        Some(r#"{"item_id":"widget-a"}"#),
        5000,
    )
    .unwrap();
    assert!(out.contains("\"quantity\":42"), "{out}");
    let out = pc::call(ep, "debug", "echo", Some(r#"{"x":"é"}"#), 5000).unwrap();
    assert!(out.contains("echo"), "{out}");

    let e = pc::call(ep, "debug", "no_such_op", None, 5000).unwrap_err();
    assert_eq!(e.code, status::NOT_FOUND, "{e}");
    assert!(
        e.output
            .as_deref()
            .unwrap_or("")
            .contains("operation.unknown"),
        "{e:?}"
    );

    let e = pc::call(ep, "inventory", "get", Some(r#"{"item_id":"nope"}"#), 5000).unwrap_err();
    assert_eq!(e.code, status::REMOTE, "{e}");

    let t0 = Instant::now();
    let e = pc::call(ep, "debug", "sleep", Some(r#"{"ms":3000}"#), 300).unwrap_err();
    assert_eq!(e.code, status::TIMEOUT, "{e}");
    assert!(
        t0.elapsed() < Duration::from_millis(2900),
        "deadline not enforced"
    );

    assert_eq!(
        code(pc::call(ep, "debug", "echo", Some("{bad json"), 5000)),
        status::INVALID_ARGUMENT
    );
    assert_eq!(
        code(pc::call(ep, "debug", "echo", None, 0)),
        status::INVALID_ARGUMENT
    );
    assert_eq!(
        code(pc::call("not-an-endpoint", "debug", "echo", None, 1000)),
        status::INVALID_ARGUMENT
    );
    drop(rt);

    // no runtime: a port nobody listens on
    let gone = node("gone");
    let dead_ep = gone.endpoint().unwrap();
    gone.close().unwrap();
    assert_eq!(
        code(pc::call(&dead_ep, "debug", "echo", None, 2000)),
        status::TRANSPORT
    );
}

// ------------------------------------------------------------------- peers

#[test]
fn two_nodes_exchange_both_directions() {
    let a = node("alpha");
    let b = node("beta");
    assert_eq!(a.node_id().unwrap(), "alpha");
    let (ea, eb) = (a.endpoint().unwrap(), b.endpoint().unwrap());
    assert!(ea.starts_with("127.0.0.1:") && ea != eb);
    a.register("beta", &eb).unwrap();
    b.register("alpha", &ea).unwrap();

    a.send("beta", b"hello beta", Some("m-a2b"), 5000).unwrap();
    let m = b.recv(5000).unwrap();
    assert_eq!(
        (
            m.sender.as_str(),
            m.message_id.as_str(),
            m.payload.as_slice()
        ),
        ("alpha", "m-a2b", &b"hello beta"[..])
    );

    b.send(&ea, b"hello alpha", Some("m-b2a"), 5000).unwrap();
    let m = a.recv(5000).unwrap();
    assert_eq!(
        (
            m.sender.as_str(),
            m.message_id.as_str(),
            m.payload.as_slice()
        ),
        ("beta", "m-b2a", &b"hello alpha"[..])
    );

    // generated message id
    a.send("beta", b"auto", None, 5000).unwrap();
    let m = b.recv(5000).unwrap();
    assert!(!m.message_id.is_empty());
    assert_eq!(m.payload, b"auto");

    a.ping("beta", 2000).unwrap();
    a.ping(&eb, 2000).unwrap();
    let h = b.health().unwrap();
    assert!(h.contains("\"node_id\":\"beta\""), "{h}");
}

#[test]
fn payload_shapes_and_limits() {
    let a = node("shape-a");
    let b = node("shape-b");
    let eb = b.endpoint().unwrap();
    let utf8 = "h\u{e9}llo \u{2014} \u{4e16}\u{754c} \u{1f30d}"
        .as_bytes()
        .to_vec();
    let binary: Vec<u8> = (0..=255u8).chain([0, 0, 1]).collect();
    let max: Vec<u8> = (0..pc::PEER_MAX_PAYLOAD)
        .map(|i| (i * 31 % 251) as u8)
        .collect();
    for (id, payload) in [
        ("empty", Vec::new()),
        ("utf8", utf8),
        ("bin", binary),
        ("max", max),
    ] {
        a.send(&eb, &payload, Some(id), 20000)
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        let m = b.recv(20000).unwrap();
        assert_eq!(m.message_id, id);
        assert_eq!(m.sender, "shape-a");
        assert!(
            m.payload == payload,
            "{id}: payload differs ({} vs {} bytes)",
            m.payload.len(),
            payload.len()
        );
    }
    let over = vec![7u8; pc::PEER_MAX_PAYLOAD + 1];
    let e = a.send(&eb, &over, Some("over"), 5000).unwrap_err();
    assert_eq!(e.code, status::TOO_LARGE, "{e}");
    assert_eq!(
        code(b.recv(300)),
        status::TIMEOUT,
        "oversize payload must not be delivered"
    );
}

#[test]
fn registry_is_owned_by_each_node() {
    let a = node("own-a");
    let b = node("own-b");
    let (ea, eb) = (a.endpoint().unwrap(), b.endpoint().unwrap());
    a.register("own-b", &eb).unwrap();
    assert!(a.list().unwrap().contains(&format!("\"own-b\":\"{eb}\"")));
    assert_eq!(b.list().unwrap().trim(), "{}");
    a.send("own-b", b"x", Some("r1"), 5000).unwrap();
    b.recv(5000).unwrap();
    assert_eq!(
        b.list().unwrap().trim(),
        "{}",
        "receiving must not register the sender"
    );
    assert_eq!(code(b.send("own-a", b"x", None, 2000)), status::NOT_FOUND);
    b.send(&ea, b"by endpoint", Some("r2"), 5000).unwrap();
    assert_eq!(a.recv(5000).unwrap().payload, b"by endpoint");
    a.unregister("own-b").unwrap();
    assert_eq!(code(a.unregister("own-b")), status::NOT_FOUND);
    assert_eq!(code(a.register("bad id", &eb)), status::INVALID_ARGUMENT);
    assert_eq!(code(a.register("ok", "no-port")), status::INVALID_ARGUMENT);
}

#[test]
fn duplicate_message_id_is_delivered_once() {
    let a = node("dup-a");
    let b = node("dup-b");
    let eb = b.endpoint().unwrap();
    a.send(&eb, b"once", Some("same-id"), 5000).unwrap();
    a.send(&eb, b"once", Some("same-id"), 5000).unwrap();
    assert_eq!(b.recv(5000).unwrap().message_id, "same-id");
    assert_eq!(code(b.recv(500)), status::TIMEOUT);
    let h = b.health().unwrap();
    assert!(h.contains("\"duplicates\":1"), "{h}");
}

#[test]
fn auth_failure_is_reported_and_nothing_is_queued() {
    let b = node("auth-b");
    let eb = b.endpoint().unwrap();
    let mallory = Peer::open("mallory", None, Some("wrong-token")).unwrap();
    assert_eq!(code(mallory.send(&eb, b"x", None, 3000)), status::AUTH);
    let anon = Peer::open("anon", None, None).unwrap();
    assert_eq!(code(anon.send(&eb, b"x", None, 3000)), status::AUTH);
    assert_eq!(code(b.recv(300)), status::TIMEOUT);
}

#[test]
fn send_to_dead_peer_and_timeouts() {
    let a = Peer::open("dead-a", None, Some(TOKEN)).unwrap();
    assert_eq!(a.endpoint().unwrap(), "", "send-only node has no endpoint");
    let c = node("dead-c");
    let ec = c.endpoint().unwrap();
    c.close().unwrap();
    let e = a
        .send(&ec, b"into the void", Some("void"), 2000)
        .unwrap_err();
    assert_eq!(e.code, status::TRANSPORT, "{e}");
    assert!(!e.detail.is_empty());
    assert_eq!(code(a.ping(&ec, 1000)), status::TRANSPORT);

    let b = node("timeout-b");
    assert_eq!(code(b.recv(0)), status::TIMEOUT);
    let t0 = Instant::now();
    assert_eq!(code(b.recv(300)), status::TIMEOUT);
    assert!(t0.elapsed() >= Duration::from_millis(250));
}

#[test]
fn too_small_buffer_leaves_message_queued() {
    let a = node("small-a");
    let b = node("small-b");
    a.send(&b.endpoint().unwrap(), &[9u8; 100], Some("big"), 5000)
        .unwrap();
    let e = b.recv_with_capacity(5000, 10).unwrap_err();
    assert_eq!(e.code, status::TOO_LARGE);
    assert_eq!(e.needed, Some(100));
    let m = b.recv_with_capacity(0, 100).unwrap();
    assert_eq!((m.message_id.as_str(), m.payload.len()), ("big", 100));
}

#[test]
fn cancel_and_close_wake_a_blocked_receive() {
    let b = Arc::new(node("wake-b"));
    let r = Arc::clone(&b);
    let t = std::thread::spawn(move || r.recv(WAIT_FOREVER));
    std::thread::sleep(Duration::from_millis(300));
    b.cancel().unwrap();
    assert_eq!(t.join().unwrap().unwrap_err().code, status::CANCELLED);

    let r = Arc::clone(&b);
    let t = std::thread::spawn(move || r.recv(WAIT_FOREVER));
    std::thread::sleep(Duration::from_millis(300));
    b.close().unwrap();
    assert_eq!(t.join().unwrap().unwrap_err().code, status::CLOSED);
}

#[test]
fn double_close_calls_after_close_and_invalid_handles() {
    let p = node("closing");
    let h = p.handle();
    assert!(h > 0);
    p.close().unwrap();
    assert_eq!(code(p.close()), status::INVALID_HANDLE);
    assert_eq!(code(p.endpoint()), status::INVALID_HANDLE);
    assert_eq!(
        code(p.send("127.0.0.1:9", b"x", None, 100)),
        status::INVALID_HANDLE
    );
    assert_eq!(code(p.recv(0)), status::INVALID_HANDLE);
    assert_eq!(code(p.cancel()), status::INVALID_HANDLE);
    for raw in [0, -1, 123_456, h] {
        // SAFETY: integer handles are validated by the core.
        assert_eq!(
            unsafe { sys::polycall_peer_close(raw) },
            status::INVALID_HANDLE,
            "handle {raw}"
        );
    }
    drop(p); // Drop after an explicit close must not close anything else
    assert_eq!(
        code(Peer::open("bad id!", None, None)),
        status::INVALID_ARGUMENT
    );
    assert_eq!(
        code(Peer::open("x", Some("0.0.0.0:0"), None)),
        status::CONFIG,
        "non-loopback bind without a token must be refused"
    );
}

#[test]
fn concurrent_senders() {
    let b = node("conc-b");
    let eb = b.endpoint().unwrap();
    let threads: Vec<_> = (0..8)
        .map(|t| {
            let eb = eb.clone();
            std::thread::spawn(move || {
                let s = Peer::open(&format!("sender-{t}"), None, Some(TOKEN)).unwrap();
                for i in 0..25 {
                    s.send(
                        &eb,
                        format!("{t}:{i}").as_bytes(),
                        Some(&format!("m{t}-{i}")),
                        10000,
                    )
                    .unwrap();
                }
            })
        })
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..200 {
        let m = b.recv(10000).unwrap();
        let t: usize = m.sender.trim_start_matches("sender-").parse().unwrap();
        let i: usize = m.message_id.split('-').nth(1).unwrap().parse().unwrap();
        assert_eq!(m.payload, format!("{t}:{i}").as_bytes());
        assert!(seen.insert((t, i)), "duplicate {t}:{i}");
    }
    for th in threads {
        th.join().unwrap();
    }
    assert_eq!(seen.len(), 200);
    assert_eq!(code(b.recv(200)), status::TIMEOUT);
}

// ----------------------------------------------- interop with the C CLI node

#[test]
fn interop_with_polycall_peer_serve_both_directions() {
    let c = start_c_peer("cnode");
    let r = node("rnode");

    // Rust -> C: binary payload, read back raw through the CLI
    let binary: Vec<u8> = (0..=255u8).rev().chain([0u8, 0, 0]).collect();
    r.send(&c.endpoint, &binary, Some("r2c-bin"), 5000).unwrap();
    let out = Command::new(cli())
        .args(["peer", "recv", "--to", &c.endpoint, "-t", "5000", "--raw"])
        .env("POLYCALL_DEV_TOKEN", TOKEN)
        .env("POLYCALL_TELEMETRY", "off")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, binary, "C node received different bytes");

    // Rust -> C: sender id and message id verified at the C side
    let text = "from rust \u{2713}";
    r.send(&c.endpoint, text.as_bytes(), Some("r2c-text"), 5000)
        .unwrap();
    let out = Command::new(cli())
        .args(["peer", "recv", "--to", &c.endpoint, "-t", "5000"])
        .env("POLYCALL_DEV_TOKEN", TOKEN)
        .env("POLYCALL_TELEMETRY", "off")
        .output()
        .unwrap();
    let json = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{json}");
    assert!(json.contains("\"from\":\"rnode\""), "{json}");
    assert!(json.contains("\"id\":\"r2c-text\""), "{json}");
    assert!(
        json.contains(&format!("\"payload_b64\":\"{}\"", b64(text.as_bytes()))),
        "{json}"
    );

    // C -> Rust: `polycall peer send` from a file with NULs
    let payload: Vec<u8> = b"C\0to\0Rust\xff\x00".to_vec();
    let file = tmp_path("c2r.bin");
    std::fs::File::create(&file)
        .unwrap()
        .write_all(&payload)
        .unwrap();
    let rep = r.endpoint().unwrap();
    let st = Command::new(cli())
        .args([
            "peer",
            "send",
            "--from",
            "cnode",
            "--to",
            &rep,
            "--id",
            "c2r-1",
            "--payload-file",
        ])
        .arg(&file)
        .env("POLYCALL_DEV_TOKEN", TOKEN)
        .env("POLYCALL_TELEMETRY", "off")
        .output()
        .unwrap();
    assert!(
        st.status.success(),
        "{}",
        String::from_utf8_lossy(&st.stderr)
    );
    let m = r.recv(5000).unwrap();
    assert_eq!(
        (m.sender.as_str(), m.message_id.as_str()),
        ("cnode", "c2r-1")
    );
    assert_eq!(m.payload, payload);

    // the C node answers ping under its id once registered
    r.register("cnode", &c.endpoint).unwrap();
    r.ping("cnode", 2000).unwrap();
    r.register("impostor", &c.endpoint).unwrap();
    assert_eq!(
        code(r.ping("impostor", 2000)),
        status::PROTOCOL,
        "identity must be checked"
    );
}
