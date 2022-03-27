use std::{net::{Ipv4Addr, SocketAddr, IpAddr}, time::Duration};

use onvif::schema::ws_discovery::{probe, probe_matches};
use tokio::{net::UdpSocket, time};

use crate::device::DeviceError;

// Adapted from https://github.com/lumeohq/onvif-rs/blob/main/onvif/examples/discovery.rs
// Copyright (c) 2019 Lumeo, Inc.

pub async fn discover() -> Result<(), DeviceError> {
    let local_ifaces = nix::ifaddrs::getifaddrs()
    .map_err(|err| format!("Could not get local IP addresses: {}", err))?;

    let ipv4s = local_ifaces
    .filter_map( |iface| iface.address)
    .filter_map( |iface_addr| {
        if let nix::sys::socket::SockAddr::Inet(iface_addr) = iface_addr {
            return Some(iface_addr.ip().to_std());
        }
        return None;
    })
    .filter_map( |ip| {
        match ip {
            IpAddr::V4(ipv4) => Some(ipv4),
            IpAddr::V6(_) => None,
        }
    })
    .filter( |ip| !ip.is_loopback() );

    for local_ipv4 in ipv4s {
        println!("Checking {} for cameras...", local_ipv4);

        let socket = send_probe(local_ipv4).await
        .map_err(|err| format!("Could not send probe: {}", err))?;

        loop {
            let response_result = recv_string(&socket, Duration::from_millis(5000)).await;
            if response_result.is_err() {
                break; // Timeout
            }
            let response = response_result.unwrap();
            let envelope = yaserde::de::from_str::<probe_matches::Envelope>(&response).ok().unwrap();

            let envelope_iter = envelope
                .body
                .probe_matches
                .probe_match
                .iter()
                .filter(|probe_match| {
                    probe_match
                        .find_in_scopes("onvif://www.onvif.org")
                        .is_some()
                });
            
            for probe_match in envelope_iter {
                println!("{:?}", probe_match);
            }
        }
    }

    return Ok(());
}

async fn recv_string(s: &UdpSocket, timeout: Duration) -> tokio::io::Result<String> {
    let mut buf = vec![0; 16 * 1024];
    let (len, _src) = time::timeout(timeout, s.recv_from(&mut buf)).await??;

    Ok(String::from_utf8_lossy(&buf[..len]).to_string())
}

fn build_probe() -> probe::Envelope {
    use probe::*;

    Envelope {
        header: Header {
            message_id: format!("uuid:{}", uuid::Uuid::new_v4()),
            action: "http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe".into(),
            to: "urn:schemas-xmlsoap-org:ws:2005:04:discovery".into(),
        },
        ..Default::default()
    }
}

async fn send_probe(from_addr: Ipv4Addr) -> tokio::io::Result<UdpSocket> {
    let probe = build_probe();
    let probe_xml = yaserde::ser::to_string(&probe).unwrap();//.map_err(Error::Serde)?;

    const LOCAL_PORT: u16 = 0;

    const MULTI_IPV4_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
    const MULTI_PORT: u16 = 3702;

    let local_socket_addr = SocketAddr::new(IpAddr::V4(from_addr), LOCAL_PORT);
    let multi_socket_addr = SocketAddr::new(IpAddr::V4(MULTI_IPV4_ADDR), MULTI_PORT);

    let socket = UdpSocket::bind(local_socket_addr).await?;
    socket.join_multicast_v4(MULTI_IPV4_ADDR, from_addr)?;

    socket.send_to(probe_xml.as_bytes(), multi_socket_addr).await?;

    return Ok(socket);
}