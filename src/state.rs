use {
    crate::{
        acks::Acks,
        broadcasts::{Broadcast, Broadcasts},
        peer::Peer,
    },
    futures::lock::Mutex,
    indexmap::IndexMap,
    log::{debug, info},
    rand::{seq::IteratorRandom, thread_rng},
    std::{net::SocketAddr, time::Duration},
    uuid::Uuid,
};

#[derive(Debug, PartialEq, Clone)]
pub enum NodeState {
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub struct State {
    id: Mutex<Uuid>,
    peers: Mutex<IndexMap<SocketAddr, Peer>>,
    state: Mutex<NodeState>,
    broadcasts: Mutex<Broadcasts>,
    acks: Mutex<Acks>,
}

impl State {
    pub fn new() -> Self {
        State {
            id: Mutex::new(Uuid::new_v4()),
            peers: Mutex::new(IndexMap::new()),
            state: Mutex::new(NodeState::Disconnected),
            broadcasts: Mutex::new(Broadcasts::new()),
            acks: Mutex::new(Acks::new(Duration::from_secs(2))),
        }
    }

    pub fn peers(&self) -> &Mutex<IndexMap<SocketAddr, Peer>> {
        &self.peers
    }

    pub fn id(&self) -> &Mutex<Uuid> {
        &self.id
    }

    pub fn acks(&self) -> &Mutex<Acks> {
        &self.acks
    }

    pub fn broadcasts(&self) -> &Mutex<Broadcasts> {
        &self.broadcasts
    }

    pub async fn update_state(&self, state: NodeState) {
        let mut current_state = await!(self.state.lock());
        std::mem::replace(&mut *current_state, state);
    }

    pub async fn peers_sync(&self, incoming_peers: Vec<(SocketAddr, Uuid)>) {
        let mut peers = await!(self.peers.lock());

        for (addr, id) in incoming_peers {
            if !peers.contains_key(&addr) {
                debug!("Adding peer: {:?}, from: {:?}", id, addr);
                let peer = Peer::new_alive(id.clone(), addr);
                peers.insert(addr, peer);
            }
        }
    }

    pub async fn get_peer(&self) -> Option<Peer> {
        let peers = await!(self.peers.lock());
        let mut rng = thread_rng();
        peers.iter().choose(&mut rng).map(|(_, peer)| peer.clone())
    }

    pub async fn peer_join(&self, id: Uuid, addr: SocketAddr) {
        let peer = Peer::new_alive(id, addr);
        let mut peers = await!(self.peers.lock());
        peers.insert(peer.addr(), peer);
        await!(self.add_broadcast(Broadcast::Joined(id, addr)));
    }

    pub async fn add_broadcast(&self, broadcast: Broadcast) {
        let mut broadcasts = await!(self.broadcasts().lock());
        broadcasts.add(broadcast);
    }

    pub async fn apply_broadcasts(&self, broadcasts: Vec<Broadcast>) {
        for broadcast in broadcasts {
            match broadcast {
                Broadcast::Joined(id, addr) => {
                    let our_id = await!(self.id.lock());

                    if id != *our_id {
                        let mut peers = await!(self.peers().lock());
                        if !peers.contains_key(&addr) {
                            // TODO: check to see if the addr matches what we know of that peer
                            let peer = Peer::new_alive(id, addr);

                            info!("Peer: {} has joined", peer.id().to_string());

                            peers.insert(peer.addr(), peer);
                            await!(self.add_broadcast(Broadcast::Joined(id, addr)));
                        }
                    } else {
                        await!(self.add_broadcast(Broadcast::Joined(id, addr)));
                    }
                }
                //_ => unimplemented!(),
            }
        }
    }
}
