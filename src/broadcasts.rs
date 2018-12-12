use {
    serde_derive::{Deserialize, Serialize},
    std::{collections::VecDeque, net::SocketAddr},
    uuid::Uuid,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Broadcast {
    Joined(Uuid, SocketAddr),
}

#[derive(Debug, Default, Clone)]
pub struct Broadcasts(VecDeque<Broadcast>);

impl Broadcasts {
    pub fn drain(&mut self) -> Vec<Broadcast> {
        let max = 5.min(self.0.len());
        self.0.drain(0..max).collect()
    }

    pub fn add(&mut self, broadcast: Broadcast) {
        self.0.push_back(broadcast);
    }
}
