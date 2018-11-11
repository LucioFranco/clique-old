use bytes::{BufMut, BytesMut};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};
use tokio::codec::{Decoder, Encoder};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum Msg {
    Ping,
    Ack,
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

#[derive(Serialize, Deserialize, Debug)]
pub enum Join {
    Request(Uuid),
    Peers(HashMap<SocketAddr, Uuid>),
    Done,
}

pub struct JoinCodec;

impl Decoder for JoinCodec {
    type Item = Join;
    type Error = std::io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> std::io::Result<Option<Join>> {
        if buf.len() > 0 {
            let decode_msg = serde_json::from_slice(&buf[..])?;
            Ok(Some(decode_msg))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for JoinCodec {
    type Item = Join;
    type Error = std::io::Error;

    fn encode(&mut self, data: Join, buf: &mut BytesMut) -> std::io::Result<()> {
        let bytes = serde_json::to_vec(&data)?;
        buf.put(&bytes[..]);
        Ok(())
    }
}
