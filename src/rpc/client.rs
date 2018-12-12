use {
    crate::{
        error::{Error, Result},
        rpc::proto::{client::Member, Pull, Push},
    },
    futures::compat::Future01CompatExt,
    http::Uri,
    std::net::SocketAddr,
    tokio::{executor::DefaultExecutor, net::TcpStream},
    tower_grpc::{BoxBody, Request},
    tower_h2::client::Connection,
    tower_http::{add_origin, AddOrigin},
};

pub struct Client {
    client: Member<AddOrigin<Connection<TcpStream, DefaultExecutor, BoxBody>>>,
}

impl Client {
    pub async fn connect(addr: &SocketAddr, origin: Uri) -> Result<Client> {
        let socket =
            await!(TcpStream::connect(addr).compat()).expect("Unable to create the TcpStream");

        let conn = {
            let conn = await!(Connection::handshake(socket, DefaultExecutor::current()).compat())
                .expect("Unable to create the connection");

            add_origin::Builder::new()
                .uri(origin)
                .build(conn)
                .expect("Unable to add origin")
        };

        Ok(Client {
            client: Member::new(conn),
        })
    }

    pub async fn join(&mut self, push: Push) -> Result<Pull> {
        let request = self.client.join(Request::new(push));

        let response = await!(request.compat());
        response.map(|e| e.into_inner()).map_err(Error::from)
    }
}
