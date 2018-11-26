use indexmap::IndexMap;
use log::{info, trace};
use std::{
    net::SocketAddr,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use uuid::Uuid;

use crate::{
    broadcasts::{Broadcast, Broadcasts},
    peer::Peer,
};

#[derive(Debug, PartialEq, Clone)]
pub enum NodeState {
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub struct State {
    id: RwLock<Uuid>,
    peers: RwLock<IndexMap<Uuid, Peer>>,
    state: RwLock<NodeState>,
    broadcasts: RwLock<Broadcasts>,
}

impl State {
    pub fn new() -> Self {
        State {
            id: RwLock::new(Uuid::new_v4()),
            peers: RwLock::new(IndexMap::new()),
            state: RwLock::new(NodeState::Disconnected),
            broadcasts: RwLock::new(Broadcasts::new()),
        }
    }

    pub fn peers(&self) -> RwLockReadGuard<'_, IndexMap<Uuid, Peer>> {
        self.peers.read().expect("Peers lock poisoned")
    }

    pub fn id(&self) -> RwLockReadGuard<'_, Uuid> {
        self.id.read().expect("Unable to acquire read lock for id")
    }

    pub fn update_state(&self, state: NodeState) {
        let mut current_state = self.state.write().expect("Unable to get write lock");
        std::mem::replace(&mut *current_state, state);
    }

    pub fn peers_sync(&self, incoming_peers: Vec<(SocketAddr, Uuid)>) {
        let mut peers = self
            .peers
            .write()
            .expect("Unable to acquire write lock, in sync");

        for (addr, id) in incoming_peers {
            if !peers.contains_key(&id) {
                trace!("Adding peer: {:?}, from: {:?}", id, addr);
                let peer = Peer::new_alive(id.clone(), addr);
                peers.insert(id, peer);
            }
        }
    }

    pub fn peer_join(&self, id: Uuid, addr: SocketAddr) {
        let peer = Peer::new_alive(id, addr);

        self.insert_peer(peer);

        self.add_broadcast(Broadcast::Joined(id, addr));
    }

    pub fn insert_peer(&self, peer: Peer) {
        // TODO: this should probably update the addr if the id already exists
        self.peers
            .write()
            .expect("Unable to acquire write lock")
            .insert(peer.id(), peer);
    }

    pub fn broadcasts_mut(&self) -> RwLockWriteGuard<'_, Broadcasts> {
        self.broadcasts
            .write()
            .expect("Unable to acquire write lock")
    }

    pub fn add_broadcast(&self, broadcast: Broadcast) {
        self.broadcasts_mut().add(broadcast);
    }

    pub fn apply_broadcasts(&self, broadcasts: Vec<Broadcast>) {
        for broadcast in broadcasts {
            match broadcast {
                Broadcast::Joined(id, addr) => if !self.peers().contains_key(&id) {
                    // TODO: check to see if the addr matches what we know of that peer
                    let peer = Peer::new_alive(id, addr);

                    info!("Peer: {:?} has joined", peer);

                    self.insert_peer(peer);
                    self.add_broadcast(Broadcast::Joined(id, addr));
                },
                //_ => unimplemented!(),
            }
        }
    }
}
