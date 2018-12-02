use {
    crate::broadcasts::Broadcast,
    bytes::{BufMut, BytesMut},
    serde_derive::{Deserialize, Serialize},
    tokio::codec::{Decoder, Encoder},
};

#[derive(Serialize, Deserialize, Debug)]
pub enum Msg {
    Ping(Vec<Broadcast>),
    Ack(Vec<Broadcast>),
}

pub struct MsgCodec;

impl Decoder for MsgCodec {
    type Item = Msg;
    type Error = std::io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> std::io::Result<Option<Msg>> {
        if buf.len() > 0 {
            let decode_msg = serde_json::from_slice(&buf[..])?;
            Ok(Some(decode_msg))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for MsgCodec {
    type Item = Msg;
    type Error = std::io::Error;

    fn encode(&mut self, data: Msg, buf: &mut BytesMut) -> std::io::Result<()> {
        let bytes = serde_json::to_vec(&data)?;
        buf.put(&bytes[..]);
        Ok(())
    }
}
