//! The `gray` mount has to survive the client reconnecting. Claude Code
//! drops and reopens connections on a healthy session: a probe sibling is
//! closed as the real transport lands, and a mount that hits Claude's own
//! 30s timeout respawns `claude-sub mcp` against this same endpoint.
//! Ending the bridge the instant the session conn closed turned that churn
//! into a permanent failure the host could only report as a bare "failed".

use super::*;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::channel;
use std::time::Duration;

fn bridge() -> Bridge {
    let (tx, _rx) = channel();
    Bridge::bind(tx).expect("bind the bridge")
}

/// Connect and send the token line, exactly as `claude-sub mcp <endpoint>`
/// does before its stdio pump starts.
fn client(endpoint: &str) -> (TcpStream, BufReader<TcpStream>) {
    let rest = endpoint.strip_prefix("tcp:").expect("tcp endpoint");
    let (port, token) = rest.split_once(':').expect("port:token endpoint");
    let mut write = TcpStream::connect(("127.0.0.1", port.parse::<u16>().unwrap()))
        .expect("connect to the bridge");
    write
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("read timeout");
    write.write_all(token.as_bytes()).expect("token");
    write.write_all(b"\n").expect("token line");
    write.flush().expect("flush token");
    let read = write.try_clone().expect("clone");
    (write, BufReader::new(read))
}

/// One request/response round trip.
fn call(write: &mut TcpStream, read: &mut BufReader<TcpStream>, msg: Value) -> Value {
    write
        .write_all(format!("{msg}\n").as_bytes())
        .expect("write request");
    write.flush().expect("flush request");
    let mut line = String::new();
    read.read_line(&mut line).expect("read response");
    serde_json::from_str(&line).expect("json response")
}

/// `initialize` — what makes a conn the session transport.
fn initialize(write: &mut TcpStream, read: &mut BufReader<TcpStream>, id: Value) -> Value {
    call(
        write,
        read,
        json!({"jsonrpc": "2.0", "id": id, "method": "initialize",
            "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": "claude-code", "version": "2.1.309"}}}),
    )
}

#[test]
fn a_conn_keeps_serving_after_its_first_request() {
    // The accept loop's listener is nonblocking; on Windows the accepted
    // socket inherits that, and the first pump read after the buffered
    // request fails WouldBlock — the conn dies right after answering
    // `server/discover`, so claude never gets past its probe and the mount
    // never lands. A second request on the same conn is the tripwire.
    let b = bridge();
    let (mut w, mut r) = client(b.endpoint());
    let probe = call(
        &mut w,
        &mut r,
        json!({"jsonrpc": "2.0", "id": "p1", "method": "server/discover"}),
    );
    assert_eq!(probe["id"], json!("p1"), "discover is answered");
    let second = call(
        &mut w,
        &mut r,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    );
    assert_eq!(
        second["id"],
        json!(2),
        "the same conn keeps reading after its first request"
    );
}
