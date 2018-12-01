use http::Uri;
use std::net::SocketAddr;
use tokio::{executor::DefaultExecutor, net::TcpStream};
use tower_grpc::BoxBody;
use tower_h2::client::Connection;
use tower_http::{add_origin, AddOrigin};

use crate::rpc::proto::client::Member;

#[allow(dead_code)]
pub type Client = Member<AddOrigin<Connection<TcpStream, DefaultExecutor, BoxBody>>>;

pub async fn connect(addr: &SocketAddr, origin: Uri) -> Result<Client, ()> {
    let socket = await!(TcpStream::connect(addr)).expect("Unable to create the TcpStream");

    let conn = {
        let conn = await!(Connection::handshake(socket, DefaultExecutor::current()))
            .expect("Unable to create the connection");

        add_origin::Builder::new()
            .uri(origin)
            .build(conn)
            .expect("Unable to add origin")
    };

    Ok(Member::new(conn))
}
