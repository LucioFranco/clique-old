pub enum Error {
    Grpc(tower_grpc::Error<tower_h2::client::Error>),
}

impl From<tower_grpc::Error<tower_h2::client::Error>> for Error {
    fn from(err: tower_grpc::Error<tower_h2::client::Error>) -> Self {
        Error::Grpc(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Grpc(ref inner) => write!(f, "Grpc Error: {}", inner),
        }
    }
}
