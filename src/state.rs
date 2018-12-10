use {
    crate::{
        broadcasts::{Broadcast, Broadcasts},
        failure::Failure,
        peer::Peer,
    },
    futures::lock::Mutex,
    indexmap::IndexMap,
    log::{info, trace},
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
    peers: Mutex<IndexMap<Uuid, Peer>>,
    state: Mutex<NodeState>,
    broadcasts: Mutex<Broadcasts>,
    failures: Mutex<Failure>,
}

impl State {
    pub fn new() -> Self {
        State {
            id: Mutex::new(Uuid::new_v4()),
            peers: Mutex::new(IndexMap::new()),
            state: Mutex::new(NodeState::Disconnected),
            broadcasts: Mutex::new(Broadcasts::new()),
            failures: Mutex::new(Failure::new(Duration::from_secs(2))),
        }
    }

    pub fn peers(&self) -> &Mutex<IndexMap<Uuid, Peer>> {
        &self.peers
    }

    pub fn id(&self) -> &Mutex<Uuid> {
        &self.id
    }

    pub fn failures(&self) -> &'_ Mutex<Failure> {
        &self.failures
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
            if !peers.contains_key(&id) {
                trace!("Adding peer: {:?}, from: {:?}", id, addr);
                let peer = Peer::new_alive(id.clone(), addr);
                peers.insert(id, peer);
            }
        }
    }

    pub async fn peer_join(&self, id: Uuid, addr: SocketAddr) {
        let peer = Peer::new_alive(id, addr);
        let mut peers = await!(self.peers.lock());
        peers.insert(peer.id(), peer);
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
                    let mut peers = await!(self.peers().lock());
                    if !peers.contains_key(&id) {
                        // TODO: check to see if the addr matches what we know of that peer
                        let peer = Peer::new_alive(id, addr);

                        info!("Peer: {} has joined", peer.id().to_string());

                        peers.insert(peer.id(), peer);
                        await!(self.add_broadcast(Broadcast::Joined(id, addr)));
                    }
                }
                //_ => unimplemented!(),
            }
        }
    }
}
