//! Minimal peer process used by the cross-language interop tests.
//!
//! ```text
//! peer_tool serve --node-id ID --endpoint-file FILE [--count N] [--timeout-ms T]
//!     open a node on 127.0.0.1:0, write its endpoint to FILE, then print one
//!     line per received message: `MSG <from> <id> <payload as hex>`
//!     and exit after N messages (default 1) or T ms without one.
//! peer_tool send --from ID --to HOST:PORT --id MSG_ID --hex PAYLOAD_HEX
//!     send one payload from a send-only node; exit 0 only when acknowledged.
//! ```
//! The shared token, if any, is read from `POLYCALL_DEV_TOKEN`.

use rust_polycall::Peer;
use std::process::ExitCode;

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let token = std::env::var("POLYCALL_DEV_TOKEN").ok();
    let result = match args.first().map(String::as_str) {
        Some("serve") => serve(&args, token.as_deref()),
        Some("send") => send(&args, token.as_deref()),
        _ => Err("usage: peer_tool serve|send ...".to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("peer_tool: {e}");
            ExitCode::FAILURE
        }
    }
}

fn serve(args: &[String], token: Option<&str>) -> Result<(), String> {
    let node = arg(args, "--node-id").ok_or("--node-id required")?;
    let file = arg(args, "--endpoint-file").ok_or("--endpoint-file required")?;
    let count: usize = arg(args, "--count")
        .map_or(Ok(1), |v| v.parse())
        .map_err(|e| format!("{e}"))?;
    let timeout: u32 = arg(args, "--timeout-ms")
        .map_or(Ok(20000), |v| v.parse())
        .map_err(|e| format!("{e}"))?;
    let peer = Peer::open(&node, Some("127.0.0.1:0"), token).map_err(|e| e.to_string())?;
    let endpoint = peer.endpoint().map_err(|e| e.to_string())?;
    std::fs::write(&file, format!("{endpoint}\n")).map_err(|e| e.to_string())?;
    for _ in 0..count {
        let msg = peer.recv(timeout).map_err(|e| e.to_string())?;
        println!(
            "MSG {} {} {}",
            msg.sender,
            msg.message_id,
            hex(&msg.payload)
        );
    }
    peer.close().map_err(|e| e.to_string())
}

fn send(args: &[String], token: Option<&str>) -> Result<(), String> {
    let from = arg(args, "--from").ok_or("--from required")?;
    let to = arg(args, "--to").ok_or("--to required")?;
    let id = arg(args, "--id").ok_or("--id required")?;
    let payload = unhex(&arg(args, "--hex").unwrap_or_default()).ok_or("--hex is not hex")?;
    let peer = Peer::open(&from, None, token).map_err(|e| e.to_string())?;
    peer.send(&to, &payload, Some(&id), 10000)
        .map_err(|e| e.to_string())?;
    println!("SENT {from} {id} {}", payload.len());
    Ok(())
}
