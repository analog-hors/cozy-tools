use thiserror::Error;
use cozy_uci::UciParseError;
use cozy_uci::remark::UciRemark;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("io error: {0}")]
    IoError(#[from] tokio::io::Error),
    #[error("engine unexpectedly exited")]
    UnexpectedTermination,
    #[error("invalid message")]
    InvalidMessage(String, UciParseError),
    #[error("unexpected remark")]
    UnexpectedRemark(UciRemark),
    #[error("missing name")]
    MissingName,
    #[error("missing author")]
    MissingAuthor,
    #[error("invalid option")]
    InvalidOption
}

#[derive(Error, Debug)]
pub enum EngineAnalysisError {
    #[error("requires chess960 support")]
    Requires960
}

#[derive(Error, Debug)]
pub enum SetOptionError {
    #[error("no such option")]
    NoSuchOption,
    #[error("type mismatch")]
    TypeMismatch,
    #[error("out of range")]
    OutOfRange,
    #[error("engine error: {0}")]
    EngineError(#[from] EngineError)
}
