pub trait Transport<Message>: Send + Sync + Clone {
    type Future;

    fn send(&self, message: Message) -> Self::Future;
}
