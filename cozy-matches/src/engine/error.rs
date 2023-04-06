use thiserror::Error;
use cozy_uci::UciParseError;
use cozy_uci::remark::UciRemark;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("io error")]
    IoError(#[from] tokio::io::Error),
    #[error("missing name")]
    MissingName,
    #[error("missing author")]
    MissingAuthor,
    #[error("engine unexpectedly exited")]
    UnexpectedTermination,
    #[error("invalid message")]
    InvalidMessage(String, UciParseError),
    #[error("unexpected remark")]
    UnexpectedRemark(UciRemark)
}

#[derive(Error, Debug)]
pub enum EngineAnalysisError {
    #[error("incompatible with chess960")]
    IncompatibleWith960
}
