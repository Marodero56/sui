use async_trait::async_trait;
use bytes::Bytes;
use futures::SinkExt;
use network::{MessageHandler, Receiver, ReliableSender, Writer};
use std::error::Error;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::time::{sleep, Duration};

use crate::metrics::Metrics;

use super::agents::*;
use super::types::*;

pub struct Server<T: Agent<M>, M: Debug + Message + Send + 'static> {
    global_config: GlobalConfig, // global configuration from parsing json
    my_id: UniqueId,
    agent_type: PhantomData<T>, // type of agent living on this server
    msg_type: PhantomData<M>,   // type of message used by agent
}

impl<T: Agent<M>, M: Debug + Message + Send + 'static> Server<T, M> {
    pub fn new(global_config: GlobalConfig, my_id: UniqueId) -> Self {
        Server {
            global_config,
            my_id,
            agent_type: PhantomData,
            msg_type: PhantomData,
        }
    }

    // Helper function to initialize Agent
    // Outputs ingress and egress channels of the Agent
    fn init_agent(
        id: UniqueId,
        conf: GlobalConfig,
        metrics: Arc<Metrics>,
    ) -> (
        T,
        mpsc::Sender<NetworkMessage>,
        mpsc::Receiver<NetworkMessage>,
    ) {
        let (in_send, in_recv) = mpsc::channel(100);
        let (out_send, out_recv) = mpsc::channel(100);
        let agent = T::new(id, in_recv, out_send, conf, metrics);
        return (agent, in_send, out_recv);
    }

    // Server main function
    pub async fn run(&mut self, metrics: Arc<Metrics>) {
        // Initialize map from id to address
        let mut addr_table: HashMap<UniqueId, SocketAddr> = HashMap::new();
        for (id, entry) in &self.global_config {
            assert!(!addr_table.contains_key(&id), "ids must be unique");
            addr_table.insert(*id, SocketAddr::new(entry.ip_addr, entry.port));
        }

        // Initialize Agent and Network Manager
        // Network manager connects to agent through channels
        // initialize agent with global_config
        let (mut agent, in_sender, out_receiver) =
            Self::init_agent(self.my_id, self.global_config.clone(), metrics);

        let network_manager = NetworkManager::new(self.my_id, addr_table, in_sender, out_receiver);

        // Run the Network Manager
        // tokio::spawn(async move {
        network_manager.run().await;
        // });

        // Run the agent
        agent.run().await;
    }
}

/*****************************************************************************************
 *                                     Network Manager                                   *
 *****************************************************************************************/

#[derive(Clone)]
struct ChannelHandler {
    deliver_to_app: Sender<NetworkMessage>,
}

#[async_trait]
impl MessageHandler for ChannelHandler {
    async fn dispatch(&self, writer: &mut Writer, message: Bytes) -> Result<(), Box<dyn Error>> {
        // Reply with an ACK.
        let _ = writer.send(Bytes::from("Ack")).await;

        // Deserialize the message.
        let message: NetworkMessage = bincode::deserialize(&message).unwrap();

        match message.payload {
            SailfishMessage::Handshake() => {
                // do not deliver to app
                Ok(())
            }
            _ => {
                // Deliver the message to the application.
                self.deliver_to_app.send(message).await.unwrap();
                Ok(())
            }
        }
    }
}

// Network Manager spawns and manages TCP connections for the server.
struct NetworkManager {
    my_id: UniqueId,
    my_addr: SocketAddr, // listening addr
    addr_table: HashMap<UniqueId, SocketAddr>,
    // channel to pipe incoming messages for server
    application_in: mpsc::Sender<NetworkMessage>,
    // channel to get outgoing messages from server, to be sent over network
    application_out: mpsc::Receiver<NetworkMessage>,
}

impl NetworkManager {
    fn new(
        my_id: UniqueId,
        addr_table: HashMap<UniqueId, SocketAddr>,
        application_in: mpsc::Sender<NetworkMessage>,
        application_out: mpsc::Receiver<NetworkMessage>,
    ) -> Self {
        NetworkManager {
            my_id,
            my_addr: *addr_table.get(&my_id).unwrap(),
            addr_table,
            application_in,
            application_out,
        }
    }

    async fn run(self) {
        Receiver::spawn(
            self.my_addr,
            ChannelHandler {
                deliver_to_app: self.application_in.clone(),
            },
        );

        let mut sender = ReliableSender::new();

        // connect to everybody
        for addr in self.addr_table.values() {
            'inner: loop {
                let ping_message = NetworkMessage {
                    src: 0,
                    dst: 0,
                    payload: SailfishMessage::Handshake {},
                };
                println!("[{}] Sending handshake to {:?}", self.my_id, addr);
                let cancel_handler = sender
                    .send(
                        *addr,
                        Bytes::from(bincode::serialize(&ping_message).unwrap()),
                    )
                    .await;
                if cancel_handler.await.is_ok() {
                    break 'inner;
                }
                sleep(Duration::from_millis(1_000)).await;
            }
        }
        println!("Done connecting to everybody");
        sleep(Duration::from_millis(10_000)).await;

        let mut application_out = self.application_out;
        // check from messages from app and send them out
        tokio::spawn(async move {
            while let Some(message) = application_out.recv().await {
                let mut message = message;
                message.src = self.my_id; // set source to self
                let dst = message.dst;
                if dst == self.my_id {
                    self.application_in
                        .send(message)
                        .await
                        .expect("send to self failed");
                } else {
                    // get address from id
                    let address = self.addr_table.get(&dst).unwrap();
                    let cancel_handler = sender
                        .send(*address, Bytes::from(bincode::serialize(&message).unwrap()))
                        .await;
                    cancel_handler.await.unwrap();
                }
            }
        });
    }
}
