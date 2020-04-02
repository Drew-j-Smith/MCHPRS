mod packets;

use std::io::{self, BufReader, BufRead};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use packets::PacketDecoder;

#[derive(PartialEq)]
enum NetworkState {
    Handshake,
    Login,
    Play
}

/// This struct represents a TCP Client
struct NetworkClient {
    /// All NetworkClients are identified by this id
    id: u32,
    reader: BufReader<TcpStream>,
    state: NetworkState,
    packets: Vec<PacketDecoder>
}

impl NetworkClient {

    fn update(&mut self) {
        let incoming_data = Vec::from(match self.reader.fill_buf() {
            Ok(data) => data,
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::WouldBlock => &[],
                    _ => panic!(e)
                }
            },
        });
        let data_length = incoming_data.len();
        let mut incoming_packets = PacketDecoder::decode(false, incoming_data);
        if incoming_packets.is_empty() {
            self.packets.append(&mut incoming_packets);
        }
        self.reader.consume(data_length);
    }

}

/// This represents the network portion of a minecraft server
pub struct NetworkServer {
    client_receiver: mpsc::Receiver<NetworkClient>,
    /// These clients are either in the handshake or login state, once they shift to play, they will be moved to a plot
    clients: Vec<NetworkClient>,
}

impl NetworkServer {
    fn listen(bind_address: &str, sender: mpsc::Sender<NetworkClient>) {
        let listener = TcpListener::bind(bind_address).unwrap();

        for (index, stream) in listener.incoming().enumerate() {
            let stream = stream.unwrap();
            stream.set_nonblocking(true).unwrap();
            sender
                .send(NetworkClient {
                    // The index will increment after each client making it unique. We'll just use this as the id.
                    id: index as u32,
                    reader: BufReader::new(stream),
                    state: NetworkState::Handshake,
                    packets: Vec::new()
                })
                .unwrap();
        }
    }

    /// Creates a new NetworkServer. The server will then start accepting TCP clients.
    pub fn new(bind_address: String) -> NetworkServer {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || NetworkServer::listen(&bind_address, sender));
        NetworkServer {
            client_receiver: receiver,
            clients: Vec::new(),
        }
    }

    pub fn update(&mut self) {
        loop {
            match self.client_receiver.try_recv() {
                Ok(client) => self.clients.push(client),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    panic!("Client receiver channel disconnected!")
                }
            }
        }
        for client in self.clients.iter_mut() {
            client.update();
        }
    }
}