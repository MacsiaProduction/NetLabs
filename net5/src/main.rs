mod config;
mod error;

use crate::config::Config;
use crate::error::{Error, SocksError};
use clap::{Parser};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tokio::{io, net};
use tracing::{debug, error, error_span, field, info, warn, Instrument, Span};

#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long, env = "KOBLAS_ADDRESS", default_value_t = IpAddr::from([127, 0, 0, 1]))]
    addr: IpAddr,
    #[arg(short, long, env = "KOBLAS_PORT", default_value_t = 1080)]
    port: u16,
    #[arg(short, long, env = "KOBLAS_LIMIT", default_value_t = 127)]
    limit: i32,
    #[arg(long, env = "KOBLAS_ANONYMIZATION")]
    anon: bool,
    #[arg(short, long, env = "KOBLAS_USERS_PATH", value_name = "FILE")]
    users: Option<PathBuf>,
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(EnvFilter::from_default_env())
        .with(ErrorLayer::default())
        .init();
}

fn main() -> color_eyre::Result<()> {
    let cli = Cli::parse();

    install_tracing();
    color_eyre::install()?;

    debug!("{cli:?}");

    let config = cli.users.as_ref().map_or_else(
        || {
            warn!("users file path not set");
            Ok(Config::default())
        },
        |path| {
            if path.exists() {
                Config::from_path(path)
            } else {
                warn!("users file doesn't exist");
                Ok(Config::default())
            }
        },
    )?;

    debug!("loaded {} users", config.users.len());

    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(run(cli))
}

async fn run(cli: Cli) -> color_eyre::Result<()> {
    let listener = TcpListener::bind((cli.addr, cli.port)).await?;

    let cli = Arc::new(cli);
    let clients = Arc::new(AtomicI32::new(0));

    loop {
        let (mut stream, addr) = listener.accept().await?;

        if clients.load(Ordering::SeqCst) >= cli.limit {
            let _ = stream.shutdown().await;
            continue;
        }

        let cli = cli.clone();
        let clients = clients.clone();

        clients.fetch_add(1, Ordering::SeqCst);

        tokio::task::spawn(async move {
            let span = if cli.anon {
                Span::none()
            } else {
                error_span!(
                    "client",
                    %addr,
                    peer = field::Empty,
                    user = field::Empty
                )
            };

            async {
                info!("connected");

                if let Err(err) = handle(&mut stream).await {
                    error!("{err}");
                }

                clients.fetch_sub(1, Ordering::SeqCst);

                info!("disconnected");
            }
                .instrument(span)
                .await;

            stream.shutdown().await
        });
    }
}

const AUTH_METHOD: u8 = 0x2;
const NO_AUTH_METHOD: u8 = 0x0;
const NO_METHOD: u8 = 0xff;
const SOCKS_VERSION: u8 = 0x5;
const SUCCESS_REPLY: u8 = 0x0;

async fn handle(stream: &mut TcpStream) -> error::Result<()> {
    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf).await?;

    let ver = buf[0];
    if ver != SOCKS_VERSION {
        return Err(Error::InvalidVersion {
            expected: SOCKS_VERSION,
            found: ver,
        });
    }

    let len = buf[1] as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;

    let method = *buf
        .iter()
        .find(|&&m| { m == NO_AUTH_METHOD || m == AUTH_METHOD })
        .unwrap_or(&NO_METHOD);

    let buf = [SOCKS_VERSION, method];
    stream.write_all(&buf).await?;

    match method {
        AUTH_METHOD => return Err(Error::MethodNotFound),
        NO_METHOD => return Err(Error::MethodNotFound),
        _ => {}
    }

    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).await?;

    let ver = buf[0];
    if ver != SOCKS_VERSION {
        return Err(Error::InvalidVersion {
            expected: SOCKS_VERSION,
            found: ver,
        });
    }

    let mut reply = SUCCESS_REPLY;
    let res = socks(stream, buf).await;
    if let Err(ref err) = res {
        reply = match err {
            SocksError::InvalidAddr { .. } => 0x8,
            SocksError::InvalidCommand { .. } => 0x7,
            _ => 0x1,
        }
    }

    let buf = [SOCKS_VERSION, reply, 0, IPV4_TYPE, 0, 0, 0, 0, 0, 0];

    stream.write_all(&buf).await?;

    let mut peer = res?;
    if let Ok(addr) = peer.peer_addr() {
        let span = Span::current();
        span.record("peer", field::display(addr));
    }

    let (sent, received) = io::copy_bidirectional(stream, &mut peer).await?;
    info!("sent {sent} bytes and received {received} bytes");

    Ok(())
}

const IPV4_TYPE: u8 = 0x1;
const IPV6_TYPE: u8 = 0x4;
const DOMAIN_TYPE: u8 = 0x3;
const CONNECT_COMMAND: u8 = 0x1;

async fn socks(stream: &mut TcpStream, buf: [u8; 4]) -> Result<TcpStream, SocksError> {
    let cmd = buf[1];
    if cmd != CONNECT_COMMAND {
        return Err(SocksError::InvalidCommand {
            expected: CONNECT_COMMAND,
            found: cmd,
        });
    }

    let addr = buf[3];
    let dest = match addr {
        IPV4_TYPE => {
            let mut octets = [0u8; 4];
            stream.read_exact(&mut octets).await?;

            let port = stream.read_u16().await?;
            vec![SocketAddr::new(IpAddr::from(octets), port)]
        }
        DOMAIN_TYPE => {
            let len = stream.read_u8().await? as usize;
            let mut buf = vec![0u8; len];
            stream.read_exact(&mut buf).await?;

            let domain = String::from_utf8(buf)?;
            let port = stream.read_u16().await?;

            net::lookup_host(format!("{domain}:{port}"))
                .await?
                .collect()
        }
        IPV6_TYPE => {
            let mut octets = [0u8; 16];
            stream.read_exact(&mut octets).await?;

            let port = stream.read_u16().await?;
            vec![SocketAddr::new(IpAddr::from(octets), port)]
        }
        _ => {
            return Err(SocksError::InvalidAddr {
                expected: vec![IPV4_TYPE, DOMAIN_TYPE, IPV6_TYPE],
                found: addr,
            })
        }
    };

    Ok(TcpStream::connect(&dest[..]).await?)
}