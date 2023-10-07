use std::net::{Ipv4Addr, TcpStream};
use std::io::{self, Read, Write};
use std::fs::File;
use std::path::{PathBuf};

const BUFFER_SIZE: usize = 8192;

fn send_file(server_addr: &str, server_port: u16, file_path: &str) -> io::Result<()> {
    let binding = PathBuf::from(file_path);
    let file_name = binding
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    let file_name_len:u8 = file_name.len() as u8;
    let file = File::open(file_path)?;
    let file_size = file.metadata()?.len();

    let mut stream = TcpStream::connect((server_addr, server_port))?;
    let mut buffer = vec![0; BUFFER_SIZE];

    // Send file len, file name, size, and contents to the server
    stream.write_all(&file_name_len.to_le_bytes())?;
    stream.write_all(file_name.as_bytes())?;
    stream.write_all(&(file_size).to_le_bytes())?;

    let mut bytes_sent = 0;
    loop {
        let bytes_read = file.try_clone().expect("failed to reopen file").take(BUFFER_SIZE as u64).read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        stream.write_all(&buffer[..bytes_read])?;
        bytes_sent += bytes_read as u64;
    }

    println!("File sent successfully. Total bytes sent: {}", bytes_sent);

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: {} <server_addr> <server_port> <file_path>", args[0]);
        return;
    }

    let server_addr: &Ipv4Addr = &args[1].parse().expect("Invalid server address");
    let server_port: u16 = args[2].parse().expect("Invalid server port");
    let file_path: &str = &args[3];

    match send_file(&*server_addr.to_string(), server_port, file_path) {
        Ok(_) => println!("File sent successfully."),
        Err(err) => eprintln!("Error sending file: {:?}", err),
    }
}
