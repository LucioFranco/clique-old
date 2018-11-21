use indexmap::IndexMap;
use log::trace;
use std::{
    collections::VecDeque,
    net::SocketAddr,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub enum NodeState {
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub struct State {
    id: RwLock<Uuid>,
    peers: RwLock<IndexMap<Uuid, SocketAddr>>,
    state: RwLock<NodeState>,
    broadcasts: RwLock<VecDeque<Vec<u8>>>,
}

impl State {
    pub fn new() -> Self {
        State {
            id: RwLock::new(Uuid::new_v4()),
            peers: RwLock::new(IndexMap::new()),
            state: RwLock::new(NodeState::Disconnected),
            broadcasts: RwLock::new(VecDeque::new()),
        }
    }

    pub fn peers(&self) -> RwLockReadGuard<IndexMap<Uuid, SocketAddr>> {
        self.peers.read().expect("Peers lock poisoned")
    }

    pub fn id(&self) -> RwLockReadGuard<Uuid> {
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
                peers.insert(id, addr);
            }
        }
    }

    pub fn insert_peer(&self, id: Uuid, addr: SocketAddr) {
        // TODO: this should probably update the addr if the id already exists
        self.peers
            .write()
            .expect("Unable to acquire write lock")
            .entry(id)
            .or_insert(addr);
    }

    pub fn broadcasts_mut(&self) -> RwLockWriteGuard<VecDeque<Vec<u8>>> {
        self.broadcasts
            .write()
            .expect("Unable to acquire write lock")
    }
}
