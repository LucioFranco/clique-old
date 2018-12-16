use {
    crate::acks::SeqNum,
    std::{
        collections::HashMap,
        net::SocketAddr,
        time::{Duration, Instant},
    },
};

#[derive(Debug)]
pub struct Failure {
    timeout: Duration,
    // TODO: move to an atomic type
    seq_num: SeqNum,
    acks: HashMap<SeqNum, (Instant, SocketAddr)>,
}

impl Failure {
    pub fn new(timeout: Duration) -> Failure {
        Failure {
            timeout,
            seq_num: 0,
            acks: HashMap::new(),
        }
    }

    pub fn add(&mut self, target: SocketAddr) -> SeqNum {
        let next_seq = self.next_seq();
        self.acks.insert(next_seq, (Instant::now(), target));
        next_seq
    }

    pub fn handle_ack(&mut self, seq: SeqNum) {
        // TODO: check that returning socketaddr matches
        self.acks.remove(&seq);
    }

    /// Gather the set of nodes that have been suspected of failure
    pub fn gather(&mut self) -> Vec<SocketAddr> {
        let now = Instant::now();
        let timeout = self.timeout;

        let mut timedout_targets = Vec::new();
        self.acks.retain(|_, (ack, target)| {
            if now.duration_since(*ack) > timeout {
                timedout_targets.push(*target);

                false
            } else {
                true
            }
        });

        timedout_targets
    }

    pub fn next_seq(&mut self) -> SeqNum {
        let new_seq_num = self.seq_num.wrapping_add(1);
        self.seq_num = new_seq_num;
        new_seq_num
    }

    pub fn len(&self) -> usize {
        self.acks.len()
    }
}

#[cfg(test)]
mod test {
    use super::Failure;
    use std::time::Duration;

    #[test]
    fn gather_empty_new() {
        let mut failure = Failure::new(Duration::from_millis(20));
        assert_eq!(failure.gather(), vec![]);
        assert_eq!(failure.len(), 0);
    }

    #[test]
    fn gather_empty_with_one() {
        let mut failure = Failure::new(Duration::from_millis(20));

        failure.add("127.0.0.1:1234".parse().unwrap());

        assert_eq!(failure.gather(), vec![]);
        assert_eq!(failure.len(), 1);
    }

    #[test]
    fn gather_one() {
        let mut failure = Failure::new(Duration::from_millis(20));

        let target = "127.0.0.1:1234".parse().unwrap();

        failure.add(target);

        std::thread::sleep(Duration::from_millis(100));

        assert_eq!(failure.gather(), vec![target.clone()]);
        assert_eq!(failure.len(), 0);
    }
}
