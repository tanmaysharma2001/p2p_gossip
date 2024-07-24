use std::{process};
use std::net::{SocketAddr};
use std::time::Instant;
use p2p_gossip::{Config};
use p2p_gossip::Node;

#[tokio::main]
async fn main() {

    let start_time = Instant::now();

    let config: Config = Config::build().unwrap_or_else(|err| {
        eprintln!("Problems parsing arguments: {err}");
        process::exit(1);
    });

    let mut peer_node = Node::new(&config);

    if !config.host_address.is_empty() {

        // master
        let connect_addr = config.host_address.parse::<SocketAddr>().unwrap();

        // connect
        peer_node.connect(start_time, connect_addr);
    }

    // start
    peer_node.start(start_time)

}