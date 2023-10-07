use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::thread;
use serde_derive::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct DiscoveryMessage {
    address: String,
    port: u16,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: -s address> <port>\n or -l <address>");
        return;
    }

    let address: Ipv4Addr = args[2].parse().expect("Invalid address specified.");

    match args[1].as_str() {
        "-l" => receive_messages(address),
        "-r" => {
            let port: u16 = args[3].parse().expect("Invalid port specified.");
            send_discovery_messages(address, port)
        },
        _ => {
            println!("Usage: -s <address> <port>\n or -l <address> <port>");
            return;
        }
    }
}

fn send_discovery_messages(address:Ipv4Addr, port: u16) {
    let multicast_group = SocketAddrV4::new(Ipv4Addr::new(225, 0, 0, 1), 80);

    let udp_socket = UdpSocket::bind(format!("{}:{}", address, port)).expect("Failed to bind UDP socket");
    udp_socket.join_multicast_v4(&multicast_group.ip(), &Ipv4Addr::new(0, 0, 0, 0)).expect("Failed to join multicast group");

    let discovery_message = DiscoveryMessage {
        address: address.to_string(),
        port,
    };
    let message_json = serde_json::to_string(&discovery_message).expect("Failed to serialize message");

    loop {
        udp_socket
            .send_to(message_json.as_bytes(), multicast_group)
            .expect("Failed to send message");
        thread::sleep(Duration::from_secs(5));
    }
}

const WINDOW_DURATION: u64 = 5; // 5 seconds

fn receive_messages(address:Ipv4Addr) {

    let receive_socket = UdpSocket::bind(format!("{}:{}", address, 80)).expect("Failed to bind UDP socket");
    receive_socket
        .join_multicast_v4(&Ipv4Addr::new(225, 0, 0, 1), &address)
        .expect("Failed to join multicast group");
    let mut buffer = [0u8; 1024];
    let mut message_count_window: VecDeque<Instant> = VecDeque::new();

    loop {
        if let Ok(size) = receive_socket.recv(&mut buffer)  {
            let received_data = String::from_utf8_lossy(&buffer[..size]);

            if let Ok(discovery_message) = serde_json::from_str::<DiscoveryMessage>(&received_data) {
                println!(
                    "Received discovery message: IP: {}, Port: {}",
                    discovery_message.address, discovery_message.port
                );

                // Add the timestamp of message reception to the window
                message_count_window.push_back(Instant::now());

                // Remove timestamps older than the window duration
                let window_start = Instant::now() - Duration::new(WINDOW_DURATION, 0);
                while let Some(front_timestamp) = message_count_window.front() {
                    if *front_timestamp < window_start {
                        message_count_window.pop_front();
                    } else {
                        break;
                    }
                }

                // Print the count of messages received in the last 5 seconds
                println!("Received messages in the last 5 seconds: {}", message_count_window.len());
            }
        }
    }
}
