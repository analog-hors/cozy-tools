use std::collections::BTreeMap;
use std::path::Path;

use futures_core::stream::Stream;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use vampirc_uci::{UciMessage, UciOptionConfig};

use crate::game::ChessGame;

mod uci_convert;
mod raw_engine;
mod analysis;

use uci_convert::*;

use raw_engine::RawEngine;

pub use self::analysis::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub path: String,
    pub args: Vec<String>,
    pub options: BTreeMap<String, Option<UciOptionValue>>,
    #[serde(default)]
    pub allow_invalid_options: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UciOptionValue {
    String(String),
    Integer(i64),
    Boolean(bool),
}

#[derive(Debug)]
pub struct Engine {
    config: EngineConfig,
    engine: RawEngine,
    engine_name: String,
    engine_author: String,
    options: BTreeMap<String, UciOptionConfig>
}

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("io error")]
    IoError(#[from] tokio::io::Error),
    #[error("engine unexpectedly exited")]
    UnexpectedTermination,
    #[error("unexpected message")]
    UnexpectedMessage(String)
}

impl Engine {
    pub async fn new(config: EngineConfig) -> Result<(Self, Vec<EngineError>), EngineError> {
        let mut engine = RawEngine::new(Path::new(&config.path), &config.args).await?;
        let mut errors = Vec::new();

        let mut engine_name = None;
        let mut engine_author = None;
        let options = BTreeMap::new();
        engine.send(UciMessage::Uci).await?;
        loop {
            match engine.recv().await? {
                Some(UciMessage::UciOk) => break,
                Some(UciMessage::Id { name, author }) => {
                    if let Some(name) = name {
                        if engine_name.is_none() {
                            engine_name = Some(name);
                        } else {
                            errors.push(EngineError::UnexpectedMessage(format!("{}", name)));
                        }
                    }
                    if let Some(author) = author {
                        if engine_author.is_none() {
                            engine_author = Some(author);
                        } else {
                            errors.push(EngineError::UnexpectedMessage(format!("{}", author)));
                        }
                    }
                }
                //TODO handle
                Some(UciMessage::Option(_)) => {},
                Some(message) => errors.push(EngineError::UnexpectedMessage(format!("{}", message))),
                None => return Err(EngineError::UnexpectedTermination)
            }
        }
        //TODO handle
        let engine_name = engine_name.unwrap_or_default();
        let engine_author = engine_author.unwrap_or_default();

        for (name, value) in &config.options {
            let message = uci_option_to_set_option_message(name.clone(), value.as_ref());
            engine.send(message).await?;
        }

        Ok((Self {
            config,
            engine,
            engine_name,
            engine_author,
            options
        }, errors))
    }

    pub fn analyze(&mut self, game: &ChessGame, limit: AnalysisLimit) -> impl Stream<Item = Result<EngineAnalysisEvent, EngineError>> + '_ {
        let board = game.board().clone();
        let uci_position = game_to_position_message(game);
        async_stream::try_stream! {
            self.engine.send(uci_position).await?;
            self.engine.send(analyis_limit_to_go_message(limit)).await?;
            loop {
                match self.engine.recv().await? {
                    Some(UciMessage::Info(info)) => {
                        let info = EngineAnalysisInfo::from_uci(info, &board, false);
                        yield EngineAnalysisEvent::Info(info);
                    }
                    Some(UciMessage::BestMove { best_move, .. }) => {
                        let best_move = best_move.uci_move_into(&board, false);
                        yield EngineAnalysisEvent::BestMove(best_move);
                        break;
                    }
                    // TODO handle better
                    Some(message) => yield EngineAnalysisEvent::UnexpectedMessage(format!("{}", message)),
                    None => Err(EngineError::UnexpectedTermination)?,
                }
            }
        }
    }
}
