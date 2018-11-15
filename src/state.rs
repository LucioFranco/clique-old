use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub enum NodeState {
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub struct State {
    pub(crate) id: Uuid,
    pub(crate) peers: Arc<RwLock<HashMap<SocketAddr, Uuid>>>,
    pub(crate) node_state: NodeState,
}

impl State {
    pub fn new() -> Self {
        State {
            id: Uuid::new_v4(),
            peers: Arc::new(RwLock::new(HashMap::new())),
            node_state: NodeState::Disconnected,
        }
    }
}
