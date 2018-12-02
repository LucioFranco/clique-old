use {std::net::SocketAddr, uuid::Uuid};

#[derive(Debug, Clone)]
pub enum PeerState {
    Alive,
    Dead,
    Suspected,
}

#[derive(Debug, Clone)]
pub struct Peer {
    id: Uuid,
    addr: SocketAddr,
    state: PeerState,
}

impl Peer {
    pub fn new_alive(id: Uuid, addr: SocketAddr) -> Self {
        Peer {
            id,
            addr,
            state: PeerState::Alive,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn state(&self) -> PeerState {
        self.state.clone()
    }
}
