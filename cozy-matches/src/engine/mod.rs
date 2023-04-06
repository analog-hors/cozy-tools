use std::path::Path;

use cozy_uci::UciFormatOptions;
use cozy_uci::remark::{UciRemark, UciIdInfo};
use cozy_uci::command::UciCommand;
use tokio_stream::Stream;

use crate::game::ChessGame;

mod uci_convert;
mod raw_engine;
mod error;
mod analysis;

use uci_convert::*;
use error::{EngineError, EngineAnalysisError};
use raw_engine::RawEngine;
use analysis::{AnalysisLimit, EngineAnalysisEvent};

#[derive(Debug)]
pub struct Engine {
    engine: RawEngine,
    engine_name: String,
    engine_author: String,
}

impl Engine {
    pub async fn new(path: &Path, args: &[String]) -> Result<(Self, Vec<EngineError>), EngineError> {
        let mut this = Self {
            engine: RawEngine::new(path, args).await?,
            engine_name: String::new(),
            engine_author: String::new(),
        };
        let errors = this.init().await?;
        Ok((this, errors))
    }

    async fn init(&mut self) -> Result<Vec<EngineError>, EngineError> {
        let mut warnings = Vec::new();
        let mut engine_name = None;
        let mut engine_author = None;
        self.send(&UciCommand::Uci).await?;
        loop {
            match self.recv().await?.ok_or(EngineError::UnexpectedTermination)? {
                UciRemark::UciOk => break,
                UciRemark::Id(UciIdInfo::Name(name)) if engine_name.is_none() => {
                    engine_name = Some(name);
                }
                UciRemark::Id(UciIdInfo::Author(author)) if engine_author.is_none() => {
                    engine_author = Some(author);
                }
                UciRemark::Option { .. } => {}, //TODO handle
                rmk => warnings.push(EngineError::UnexpectedRemark(rmk)),
            }
        }
        if engine_name.is_none() {
            warnings.push(EngineError::MissingName);
        }
        if engine_author.is_none() {
            warnings.push(EngineError::MissingAuthor);
        }
        self.engine_name = engine_name.unwrap_or_default();
        self.engine_author = engine_author.unwrap_or_default();
        Ok(warnings)
    }

    fn uci_format_opts(&self) -> UciFormatOptions {
        UciFormatOptions::default() //TODO
    }

    pub(super) async fn send(&mut self, cmd: &UciCommand) -> Result<(), EngineError> {
        self.engine.send(cmd, &self.uci_format_opts()).await
    }

    pub(super) async fn recv(&mut self) -> Result<Option<UciRemark>, EngineError> {
        self.engine.recv(&self.uci_format_opts()).await
    }

    pub fn analyze(&mut self, game: &ChessGame, limit: AnalysisLimit) -> Result<EngineAnalysis<'_>, EngineAnalysisError> {
        if game.needs_chess960() { //TODO
            Err(EngineAnalysisError::IncompatibleWith960)?;
        }
        let position_cmd = game_to_position_message(game, false); //TODO
        let go_cmd = analysis_limit_to_go_message(limit);
        let stream = Box::new(async_stream::try_stream! {
            self.send(&position_cmd).await?;
            self.send(&go_cmd).await?;
            loop {
                match self.recv().await? {
                    Some(UciRemark::Info(info)) => {
                        //TODO
                        yield EngineAnalysisEvent::Info(info);
                    }
                    Some(UciRemark::BestMove { mv, .. }) => {
                        //TODO
                        yield EngineAnalysisEvent::BestMove(mv);
                        break;
                    }
                    // TODO handle better
                    Some(rmk) => yield EngineAnalysisEvent::EngineError(EngineError::UnexpectedRemark(rmk)),
                    None => Err(EngineError::UnexpectedTermination)?,
                }
            }
        });
        Ok(EngineAnalysis { stream })
    }
}

pub struct EngineAnalysis<'s> {
    stream: Box<dyn Stream<Item = Result<EngineAnalysisEvent, EngineError>> + 's>
}
