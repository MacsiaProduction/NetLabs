use std::fs::{create_dir_all, File};
use std::net::{TcpListener, TcpStream, Ipv4Addr};
use std::io::{Read, Write};
use std::{io, thread};
use std::time::{Duration, Instant};

const BUFFER_SIZE: usize = 8192;
const PRINT_INTERVAL: Duration = Duration::from_secs(3);
const OUTPUT_DIR: &str = "./received_files/";

struct ClientData {
    total_bytes_received: u64,
    start_time: Instant,
}

fn read_u8_from_stream(stream: &mut TcpStream) -> io::Result<u8> {
    let mut buffer = [0; 1];
    stream.read_exact(&mut buffer)?;
    Ok(buffer[0])
}

fn read_u64_from_stream(stream: &mut TcpStream) -> io::Result<u64> {
    let mut buffer = [0; 8];
    stream.read_exact(&mut buffer)?;
    Ok(u64::from_le_bytes(buffer))
}

fn handle_client(mut stream: TcpStream, client_id: u32) {
    let mut buffer = [0; BUFFER_SIZE];
    let mut client_data = ClientData {
        total_bytes_received: 0,
        start_time: Instant::now(),
    };

    let name_len = read_u8_from_stream(&mut stream).expect("failed to read len of name of the file");
    let mut file_name = vec![0u8; name_len as usize];
    stream.read_exact(&mut file_name).expect("failed to read file name");
    let name = String::from_utf8(file_name).expect("failed to read file name");
    println!("File name: {}", name);
    let output_dir = format!("{}{}", OUTPUT_DIR, client_id);
    create_dir_all(&output_dir).expect("Failed to create output directory");
    let file_size = read_u64_from_stream(&mut stream).expect("failed to read file size");

    let mut file = match File::create(format!("{}/{}", output_dir, name)) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Error creating file for client {}: {}", client_id, err);
            return;
        }
    };
    let mut prev_bytes_received: u64 = 0;
    let mut next_print = PRINT_INTERVAL;
    loop {
        match stream.read(&mut buffer) {
            Ok(bytes_read) if bytes_read > 0 => {
                client_data.total_bytes_received += bytes_read as u64;
                file.write_all(&buffer[..bytes_read]).unwrap();  // Write to file
            }
            Ok(0) => {
                // End of stream (client disconnected)
                println!("Client {} disconnected", client_id);
                break;
            }
            Err(err) => {
                eprintln!("Error reading from client {}: {}", client_id, err);
                break;
            }
            Ok(_) => unreachable!()
        }
        // Print speed information every PRINT_INTERVAL seconds
        if client_data.start_time.elapsed() >= next_print {
            let elapsed_secs = client_data.start_time.elapsed().as_secs() as f64;
            let average_speed = client_data.total_bytes_received as f64 / elapsed_secs;
            let instant_speed = (client_data.total_bytes_received - prev_bytes_received) as f64 / PRINT_INTERVAL.as_secs() as f64;
            let percents = client_data.total_bytes_received as f64 /file_size as f64;
            println!(
                "Client {}: Average speed: {} bytes/s & instant speed: {} bytes/s & {:.2}%",
                client_id, average_speed, instant_speed, percents * 100.0);

            prev_bytes_received = client_data.total_bytes_received;
            next_print += PRINT_INTERVAL;
        }
    }
    println!("{} bytes from {} recieved", client_data.total_bytes_received, file_size);
    println!("File received successfully for client {}", client_id);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <ip> <port>", args[0]);
        return;
    }
    let server_addr: &Ipv4Addr = &args[1].parse().expect("Invalid server address");
    let port: u16 = args[2].parse().expect("Invalid port number");

    let listener = TcpListener::bind(format!("{}:{}", server_addr, port)).expect("Failed to bind");

    let mut client_counter = 0;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                client_counter += 1;
                println!("Accepted client connection: {}", client_counter);

                let cloned_stream = stream.try_clone().expect("Failed to clone stream");
                thread::spawn(move || {
                    handle_client(cloned_stream, client_counter);
                });
            }
            Err(err) => {
                eprintln!("Error accepting connection: {:?}", err);
            }
        }
    }
}
