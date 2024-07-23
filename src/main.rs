use std::{env, process};
use std::net::{SocketAddr};
use p2p_gossip::{Config, get_current_timestamp};
use p2p_gossip::Node;

#[tokio::main]
async fn main() {

    let config: Config = Config::build().unwrap_or_else(|err| {
        eprintln!("Problems parsing arguments: {err}");
        process::exit(1);
    });

    let mut peer_node = Node::new(&config);

    if !config.host_address.is_empty() {

        println!("Not Host");

        // master
        let connect_addr = config.host_address.parse::<SocketAddr>().unwrap();

        // connect
        peer_node.connect(connect_addr);

        println!("{} - Connected to the peer at {:?}",
                 get_current_timestamp(),
                 connect_addr
        )
    }

    // start
    peer_node.start()

}