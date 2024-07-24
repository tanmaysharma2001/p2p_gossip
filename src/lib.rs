use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
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
    pub peers: Vec<SocketAddr>,
    pub period: i32,
    pub addr: SocketAddr,
}

#[derive(Serialize, Deserialize, Debug)]
enum MessageData {
    Peers(Vec<SocketAddr>),
    Message(String)
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    command: String,
    data: Option<MessageData>,
    addr: SocketAddr,
}

fn send_message(start_time: Instant, msg: &Message, addr: SocketAddr) -> Result<(), std::io::Error> {
    match &msg.data {
        Some(MessageData::Peers(ref _peers)) => {
            let message_vector = serde_json::to_vec(&msg)?;
            let mut stream = TcpStream::connect(addr)?;
            stream.write_all(&message_vector)?;
            Ok(())
        },
        Some(MessageData::Message(ref message)) => {
            println!("{:?} - Sending message {} to {:?}", format_duration(start_time.elapsed()), message, addr);

            let message_vector = serde_json::to_vec(&msg)?;
            let mut stream = TcpStream::connect(addr)?;
            stream.write_all(&message_vector)?;
            Ok(())
        },
        None => {
            let message_vector = serde_json::to_vec(&msg)?;
            let mut stream = TcpStream::connect(addr)?;
            stream.write_all(&message_vector)?;
            Ok(())
        }
    }
}

impl Node {
    pub fn new(config: &Config) -> Node {
        let addr = format!("127.0.0.1:{}", config.port).parse().expect("Unable to parse socket address.");

        Node {
            peers: Vec::new(),
            period: config.period,
            addr,
        }
    }

    pub fn connect(&mut self, start_time:Instant, addr: SocketAddr) {
        {
            let peers = self.peers.clone();
            if !peers.contains(&addr) {
                self.peers.push(addr);
            }
        }

        let message = Message {
            command: String::from(".syc"),
            data: None,
            addr: self.addr,
        };

        if let Err(e) = send_message(start_time, &message, addr) {
            eprintln!("Failed to send message to {}: {}", addr, e);
        }
    }

    fn handle_connection(
        stream: TcpStream,
        peers_clone: Arc<Mutex<Vec<SocketAddr>>>,
        node_addr: SocketAddr,
        start_time: Instant
    )
    {
        let mut reader = BufReader::new(stream);
        let mut buf = String::new();

        match reader.read_line(&mut buf) {
            Ok(0) => return, // Connection was closed
            Ok(_) => {
                let recv_msg: Message = match serde_json::from_str(&buf) {
                    Ok(msg) => msg,
                    Err(e) => {
                        eprintln!("Failed to deserialize message: {}", e);
                        return;
                    }
                };

                let mut peers= peers_clone.lock().unwrap();

                match recv_msg.command.as_str() {
                    ".syc" => {
                        // add the received node address
                        // in the peers list of parent node
                        peers.push(recv_msg.addr);

                        let peers_value = peers.clone();

                        let msg = Message {
                            command: ".upd".to_string(),
                            data: Some(MessageData::Peers(peers_value.to_owned())),
                            addr: node_addr,
                        };

                        // send to all peers
                        for peer in peers.iter() {
                            if let Err(e) = send_message(start_time, &msg, *peer) {
                                eprintln!("Failed to send message to {}: {}", peer, e);
                            }
                        }
                    }
                    ".upd" => {

                        // update the peers list
                        // of the child node
                        let recv_msg_data = recv_msg.data;

                        match recv_msg_data {
                            Some(MessageData::Peers(ref peer_list)) => {
                                for peer in peer_list.clone() {
                                    if !peers.contains(&peer) && peer != node_addr {
                                        peers.push(peer);
                                    }
                                }

                                println!("{:?} - Connected to the peers at {:?}", format_duration(start_time.elapsed()), peers.clone());
                            },
                            _ => {
                                println!("Error while updating peer list.")
                            }
                        }

                    },
                    ".random-message" => {

                        let recv_msg_data = recv_msg.data;

                        match recv_msg_data {
                            Some(MessageData::Message(ref msg)) => {
                                // Handle random message
                                println!("{:?} - Received message {:?} from {:?}", format_duration(start_time.elapsed()), msg, recv_msg.addr);
                            },
                            _ => {
                                println!("Error while updating peer list.")
                            }
                        }
                    },
                    _ => return,
                }
            }
            Err(e) => {
                eprintln!("Failed to read from stream: {}", e);
                return;
            }
        }
    }

    pub fn start(&mut self, start_time: Instant) {

        let listener = TcpListener::bind(self.addr).unwrap();
        println!("{:?} - My address is {:?}", format_duration(start_time.elapsed()), self.addr);

        let peers = Arc::new(Mutex::new(self.peers.clone()));

        // The rest of your code can go here
        // For example, you can implement periodic message sending:
        let period = self.period;
        let peers_clone_1 = Arc::clone(&peers);
        let node_addr_clone_1 = self.addr.clone();
        thread::spawn(move || {
            // Perform periodic tasks here
            loop {
                thread::sleep(std::time::Duration::from_secs(period as u64));
                let vec = peers_clone_1.lock().unwrap();
                let msg = Message {
                    command: ".random-message".to_string(),
                    data: Some(MessageData::Message("[random message]".parse().unwrap())),
                    addr: node_addr_clone_1,
                };

                for peer in vec.iter() {
                    if let Err(e) = send_message(start_time, &msg, *peer) {
                        eprintln!("Failed to send random message to {}: {}", peer, e);
                    }
                }
            }
        });

        loop {
            for stream in listener.incoming() {
                let stream = match stream {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Failed to accept connection: {}", e);
                        continue;
                    }
                };

                // create a mutex of self and then modify it after returning from connection.
                let node_addr_clone_2 = self.addr.clone();

                let peers_clone_2 = Arc::clone(&peers);

                thread::spawn(move || {
                    Self::handle_connection(
                        stream,
                        peers_clone_2,
                        node_addr_clone_2,
                        start_time
                    );
                }).join().unwrap();
            }
        }
    }
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}