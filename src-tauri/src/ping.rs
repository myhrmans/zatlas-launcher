//! Ping: how long a server takes to answer a Steam server query (A2S_INFO) on its query port, measured from this PC.
//! That's what game browsers show as ping, and it works for servers that don't answer ICMP pings. All the servers are
//! asked at once from one UDP socket, one query each.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::a2s::QUERY;

/// A server that hasn't answered by then gets no ping. Its first answer counts, whether that's its info or a
/// challenge to ask again with: either is a round trip.
const TIMEOUT: Duration = Duration::from_millis(1500);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub id: String,
    pub ip: String,
    pub query_port: u16,
}

#[derive(Serialize)]
pub struct Ping {
    pub id: String,
    /// milliseconds, or none when the server didn't answer
    pub ms: Option<u32>,
}

pub fn ping(targets: &[Target]) -> Result<Vec<Ping>, String> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("Couldn't open a socket for the ping: {e}"))?;
    socket
        .set_read_timeout(Some(Duration::from_millis(50)))
        .map_err(|e| e.to_string())?;

    // when each query was sent
    let mut sent: HashMap<SocketAddr, Instant> = HashMap::new();
    for t in targets {
        let Ok(ip) = t.ip.parse::<Ipv4Addr>() else { continue };
        let addr = SocketAddr::from((ip, t.query_port));
        if socket.send_to(QUERY, addr).is_ok() {
            sent.insert(addr, Instant::now());
        }
    }

    // the first answer from each server; errors (a timeout, or Windows reporting an earlier unreachable port) are skipped
    let mut ms: HashMap<SocketAddr, u32> = HashMap::new();
    let start = Instant::now();
    let mut buf = [0u8; 1500];
    while ms.len() < sent.len() && start.elapsed() < TIMEOUT {
        if let Ok((_, from)) = socket.recv_from(&mut buf)
            && let Some(at) = sent.get(&from)
        {
            ms.entry(from).or_insert(at.elapsed().as_millis() as u32);
        }
    }

    Ok(targets
        .iter()
        .map(|t| {
            let addr =
                t.ip.parse::<Ipv4Addr>()
                    .ok()
                    .map(|ip| SocketAddr::from((ip, t.query_port)));
            Ping {
                id: t.id.clone(),
                ms: addr.and_then(|a| ms.get(&a).copied()),
            }
        })
        .collect())
}
