pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Grpc(tower_grpc::Error<tower_h2::client::Error>),
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Grpc(ref inner) => write!(f, "Grpc Error: {}", inner),
            Error::Io(ref inner) => write!(f, "Io Error: {}", inner),
        }
    }
}

impl From<tower_grpc::Error<tower_h2::client::Error>> for Error {
    fn from(err: tower_grpc::Error<tower_h2::client::Error>) -> Self {
        Error::Grpc(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}
