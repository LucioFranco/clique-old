use indexmap::IndexMap;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock, RwLockReadGuard},
};
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub enum NodeState {
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub struct State {
    id: Arc<RwLock<Uuid>>,
    peers: Arc<RwLock<IndexMap<Uuid, SocketAddr>>>,
    state: Arc<RwLock<NodeState>>,
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            peers: self.peers.clone(),
            state: self.state.clone(),
        }
    }
}

impl State {
    pub fn new() -> Self {
        State {
            id: Arc::new(RwLock::new(Uuid::new_v4())),
            peers: Arc::new(RwLock::new(IndexMap::new())),
            state: Arc::new(RwLock::new(NodeState::Disconnected)),
        }
    }

    pub fn peers(&self) -> RwLockReadGuard<IndexMap<Uuid, SocketAddr>> {
        self.peers.read().expect("Peers lock poisoned")
    }

    pub fn update_state(&self, state: NodeState) {
        let mut current_state = self.state.write().expect("Unable to get write lock");
        std::mem::replace(&mut *current_state, state);
    }
}
