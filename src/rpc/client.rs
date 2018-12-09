use {
    crate::rpc::proto::client::Member,
    futures::compat::Future01CompatExt,
    http::Uri,
    std::net::SocketAddr,
    tokio::{executor::DefaultExecutor, net::TcpStream},
    tower_grpc::BoxBody,
    tower_h2::client::Connection,
    tower_http::{add_origin, AddOrigin},
};

#[allow(dead_code)]
pub type Client = Member<AddOrigin<Connection<TcpStream, DefaultExecutor, BoxBody>>>;

pub async fn connect(addr: &SocketAddr, origin: Uri) -> Result<Client, ()> {
    let socket = await!(TcpStream::connect(addr).compat()).expect("Unable to create the TcpStream");

    let conn = {
        let conn = await!(Connection::handshake(socket, DefaultExecutor::current()).compat())
            .expect("Unable to create the connection");

        add_origin::Builder::new()
            .uri(origin)
            .build(conn)
            .expect("Unable to add origin")
    };

    Ok(Member::new(conn))
}
