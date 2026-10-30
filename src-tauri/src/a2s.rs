//! Asking a server how full it is, with a Steam server query (A2S_INFO) to its query port. A join that waits for a
//! free slot asks this every few seconds, and the server list when a player count is pressed.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;

use serde::Serialize;

/// A2S_INFO: the header FF FF FF FF, then 'T' and "Source Engine Query". A server answers with its info, or with a
/// challenge (`A` and 4 bytes) to send back at the end of the same query.
pub const QUERY: &[u8] = b"\xFF\xFF\xFF\xFFTSource Engine Query\0";
const TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Serialize, Clone, Copy)]
pub struct Slots {
    pub players: u8,
    pub max: u8,
}

impl Slots {
    pub fn free(&self) -> bool {
        self.players < self.max
    }
}

/// The server's players and slots, or none when it doesn't answer.
pub fn slots(ip: &str, query_port: u16) -> Option<Slots> {
    let addr = SocketAddr::from((ip.parse::<Ipv4Addr>().ok()?, query_port));
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.set_read_timeout(Some(TIMEOUT)).ok()?;
    socket.connect(addr).ok()?;
    let mut buf = [0u8; 1500];
    socket.send(QUERY).ok()?;
    let mut n = socket.recv(&mut buf).ok()?;
    if n >= 9 && buf[4] == b'A' {
        let challenge = buf[5..9].to_vec();
        socket.send(&[QUERY, &challenge].concat()).ok()?;
        n = socket.recv(&mut buf).ok()?;
    }
    parse(&buf[..n])
}

/// The players and slots in an A2S_INFO answer: after the header and 'I', the protocol, four strings (name, map,
/// folder, game) and the app id.
fn parse(reply: &[u8]) -> Option<Slots> {
    if reply.len() < 6 || reply[4] != b'I' {
        return None;
    }
    let mut at = 6;
    for _ in 0..4 {
        at += reply.get(at..)?.iter().position(|b| *b == 0)? + 1;
    }
    at += 2;
    Some(Slots {
        players: *reply.get(at)?,
        max: *reply.get(at + 1)?,
    })
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn reads_players_and_slots() {
        let mut reply = b"\xFF\xFF\xFF\xFFI\x11Test Server\0chernarusplus\0dayz\0DayZ\0".to_vec();
        reply.extend_from_slice(&[0x7c, 0x56, 12, 60, 0]);
        let slots = parse(&reply).unwrap();
        assert_eq!((slots.players, slots.max), (12, 60));
        assert!(parse(b"\xFF\xFF\xFF\xFFI\x11cut off").is_none());
    }
}
