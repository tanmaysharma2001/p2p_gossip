use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug)]
pub struct Config {
    pub period: i32,
    pub port: i32,
    pub host_address: String,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    period: i32,

    #[arg(short, long, default_value_t = 8000)]
    port: i32,

    #[arg(long)]
    host_address: Option<String>,
}

impl Config {
    pub fn build() -> Result<Config, &'static str> {
        let args = Args::parse();

        Ok(Config {
            period: args.period,
            port: args.port,
            host_address: args.host_address.unwrap_or_default(),
        })
    }
}

pub struct Node {
    pub peers: Arc<Mutex<Vec<SocketAddr>>>,
    pub period: i32,
    pub addr: SocketAddr,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    command: String,
    data: Option<Vec<SocketAddr>>,
    addr: SocketAddr,
}

fn send_message(msg: &Message, addr: SocketAddr) -> Result<(), std::io::Error> {
    let message_vector = serde_json::to_vec(&msg)?;
    let mut stream = TcpStream::connect(addr)?;
    stream.write_all(&message_vector)?;
    Ok(())
}

impl Node {
    pub fn new(config: &Config) -> Node {
        let addr = format!("127.0.0.1:{}", config.port).parse().expect("Unable to parse socket address.");

        Node {
            peers: Arc::new(Mutex::new(Vec::new())),
            period: config.period,
            addr,
        }
    }

    pub fn connect(&self, addr: SocketAddr) {
        {
            let mut peers = self.peers.lock().unwrap();
            if !peers.contains(&addr) {
                peers.push(addr);
            }
        }

        let message = Message {
            command: String::from(".syc"),
            data: None,
            addr: self.addr,
        };

        if let Err(e) = send_message(&message, addr) {
            eprintln!("Failed to send message to {}: {}", addr, e);
        }
    }

    pub fn start(&self) {
        let listener = TcpListener::bind(self.addr).unwrap();

        println!("{} - My address is {:?}", get_current_timestamp(), self.addr);

        let peers = Arc::clone(&self.peers);
        let node_addr = self.addr;

        thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = match stream {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Failed to accept connection: {}", e);
                        continue;
                    }
                };

                let mut reader = BufReader::new(stream);
                let mut buf = String::new();

                match reader.read_line(&mut buf) {
                    Ok(0) => continue, // Connection was closed
                    Ok(_) => {
                        let recv_msg: Message = match serde_json::from_str(&buf) {
                            Ok(msg) => msg,
                            Err(e) => {
                                eprintln!("Failed to deserialize message: {}", e);
                                continue;
                            }
                        };

                        match recv_msg.command.as_str() {
                            ".syc" => {
                                let peers_clone = peers.lock().unwrap().clone();
                                let msg = Message {
                                    command: ".upd".to_string(),
                                    data: Some(peers_clone),
                                    addr: node_addr,
                                };

                                if let Err(e) = send_message(&msg, recv_msg.addr) {
                                    eprintln!("Failed to send message to {}: {}", recv_msg.addr, e);
                                }
                            }
                            ".random-message" => {
                                // Handle random message
                            }
                            _ => continue,
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read from stream: {}", e);
                        continue;
                    }
                }
            }
        });

        // TODO: Implement periodic message sending
    }
}

pub fn get_current_timestamp() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}