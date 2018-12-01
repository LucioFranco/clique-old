use http::Uri;
use std::net::SocketAddr;
use tokio::{executor::DefaultExecutor, net::TcpStream};
use tower_grpc::BoxBody;
use tower_h2::client::Connection;
use tower_http::{add_origin, AddOrigin};

use crate::rpc::proto::client::Member;

pub async fn connect(
    addr: &SocketAddr,
    origin: Uri,
) -> Result<Member<AddOrigin<Connection<TcpStream, DefaultExecutor, BoxBody>>>, ()> {
    let socket = await!(TcpStream::connect(addr)).unwrap();
    let conn = await!(Connection::handshake(socket, DefaultExecutor::current())).unwrap();
    let conn = add_origin::Builder::new().uri(origin).build(conn).unwrap();

    Ok(Member::new(conn))
}
