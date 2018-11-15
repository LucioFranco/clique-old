use futures::Future;
use rpc::proto::{client::Member, JoinRequest};
use std::net::SocketAddr;
use tokio::{executor::DefaultExecutor, net::TcpStream};
//use tower_grpc::client::server_streaming::ResponseFuture;
use tower_grpc::Request;
use tower_h2::client::Connection;

pub fn join(addr: &SocketAddr) -> impl Future<Item = (), Error = ()> {
    let uri: http::Uri = format!("http://localhost:8082").parse().unwrap();

    TcpStream::connect(&addr)
        .and_then(move |socket| {
            // Bind the HTTP/2.0 connection
            Connection::handshake(socket, DefaultExecutor::current())
                .map_err(|_| panic!("failed HTTP/2.0 handshake"))
        }).map(move |conn| {
            use tower_http::add_origin;

            let conn2 = add_origin::Builder::new().uri(uri).build(conn).unwrap();

            Member::new(conn2)
        }).and_then(|mut client| {
            client
                .join(Request::new(JoinRequest {
                    id: "12314".into(),
                    address: "1213123123".into(),
                })).map_err(|e| panic!("gRPC request failed; err={:?}", e))
        }).and_then(|response| {
            println!("RESPONSE = {:?}", response);
            Ok(())
        }).map_err(|e| {
            println!("ERR = {:?}", e);
        })
}
