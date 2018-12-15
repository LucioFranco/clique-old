use {
    serde_derive::{Deserialize, Serialize},
    std::{cmp::Ordering, net::SocketAddr},
    uuid::Uuid,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Broadcast {
    Joined(Uuid, SocketAddr),
}

impl Broadcast {
    pub fn invalidates(other: Broadcast) -> bool {
        unimplemented!()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Broadcasts(Vec<LimitedBroadcast>);

impl Broadcasts {
    pub fn add(&mut self, broadcast: Broadcast) {
        // TODO: check if it invalidates another broadcast
        self.0.push(LimitedBroadcast::new(broadcast));
    }

    pub fn get(&mut self) -> Vec<Broadcast> {
        // TODO: this should be log(num_nodes)
        let transmit_limit = 5;

        let mut broadcasts_to_send = Vec::new();
        let mut broadcasts_to_prune = Vec::new();

        for (i, broadcast) in self.0.iter().enumerate() {
            if broadcast.transmits > transmit_limit {
                broadcasts_to_prune.push(i);
            } else {
                broadcasts_to_send.push(broadcast.clone());
            }
        }

        for broadcast_index in broadcasts_to_prune {
            self.0.remove(broadcast_index);
        }

        self.0.sort();

        broadcasts_to_send.into_iter().map(|b| b.into()).collect()
    }
}

impl From<LimitedBroadcast> for Broadcast {
    fn from(b: LimitedBroadcast) -> Broadcast {
        b.broadcast
    }
}

#[derive(Debug, Clone)]
struct LimitedBroadcast {
    pub broadcast: Broadcast,
    pub transmits: usize,
}

impl LimitedBroadcast {
    pub fn new(broadcast: Broadcast) -> LimitedBroadcast {
        LimitedBroadcast {
            broadcast,
            transmits: 0,
        }
    }
}

impl Ord for LimitedBroadcast {
    fn cmp(&self, other: &LimitedBroadcast) -> Ordering {
        self.transmits.cmp(&other.transmits)
    }
}

impl PartialEq for LimitedBroadcast {
    fn eq(&self, other: &LimitedBroadcast) -> bool {
        self.transmits == other.transmits
    }
}

impl PartialOrd for LimitedBroadcast {
    fn partial_cmp(&self, other: &LimitedBroadcast) -> Option<Ordering> {
        Some(self.transmits.cmp(&other.transmits))
    }
}
impl Eq for LimitedBroadcast {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::next_addr;
    use uuid::Uuid;

    #[test]
    fn basic() {
        let broadcast = Broadcast::Joined(Uuid::new_v4(), next_addr());

        let mut broadcasts = Broadcasts::default();
        broadcasts.add(broadcast.clone());

        let broadcast_to_send = broadcasts.get().pop();
        assert_eq!(broadcast_to_send, Some(broadcast));
    }
}
