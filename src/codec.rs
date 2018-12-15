use {
    crate::{broadcasts::Broadcast, failure::SeqNum},
    bytes::{BufMut, BytesMut},
    serde_derive::{Deserialize, Serialize},
    tokio::codec::{Decoder, Encoder},
};

#[derive(Serialize, Deserialize, Debug)]
pub enum Msg {
    Ping(SeqNum, Vec<Broadcast>),
    Ack(SeqNum, Vec<Broadcast>),
    PingReq(SeqNum, Vec<Broadcast>),
    NAck(SeqNum, Vec<Broadcast>),
}

pub struct MsgCodec;

impl Decoder for MsgCodec {
    type Item = Msg;
    type Error = bincode::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> bincode::Result<Option<Msg>> {
        if !buf.is_empty() {
            let decode_msg = bincode::deserialize(&buf[..])?;
            Ok(Some(decode_msg))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for MsgCodec {
    type Item = Msg;
    type Error = bincode::Error;

    fn encode(&mut self, data: Msg, buf: &mut BytesMut) -> bincode::Result<()> {
        let bytes = bincode::serialize(&data)?;
        buf.put(&bytes[..]);
        Ok(())
    }
}
