/// A [Result] type that is wrapped to use the clique
/// [Error] type.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type returned by `clique`.
#[derive(Debug)]
pub enum Error {
    /// Errors returned from an RPC request.
    Grpc(tower_grpc::Error<tower_h2::client::Error>),
    /// IO Errors that happen during IO operation.
    Io(std::io::Error),
    /// Uuid parsing errors
    Uuid(uuid::parser::ParseError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Grpc(ref inner) => write!(f, "Grpc Error: {}", inner),
            Error::Io(ref inner) => write!(f, "Io Error: {}", inner),
            Error::Uuid(ref inner) => write!(f, "Uuid error: {}", inner),
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

impl From<uuid::parser::ParseError> for Error {
    fn from(err: uuid::parser::ParseError) -> Self {
        Error::Uuid(err)
    }
}
