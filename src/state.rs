use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use uuid::Uuid;

#[derive(Default, Debug)]
pub struct State {
    pub(crate) id: Uuid,
    pub(crate) peers: Arc<RwLock<HashMap<SocketAddr, Uuid>>>,
}
