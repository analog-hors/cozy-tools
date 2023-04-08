use std::collections::BTreeMap;
use std::path::Path;

use cozy_uci::UciFormatOptions;
use cozy_uci::remark::{UciRemark, UciIdInfo, UciOptionInfo};
use cozy_uci::command::UciCommand;

use crate::game::ChessGame;

mod uci_convert;
mod raw_engine;
mod error;
mod analysis;

use uci_convert::*;
pub use error::*;
pub use raw_engine::*;
pub use analysis::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UciOptionField {
    Check {
        value: bool,
    },
    Spin {
        value: i64,
        min: i64,
        max: i64,
    },
    Combo {
        value: usize,
        labels: Vec<String>,
    },
    String {
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UciOptionValue {
    Check(bool),
    Spin(i64),
    Combo(usize),
    String(String),
}

#[derive(Debug)]
pub struct Engine {
    engine: RawEngine,
    engine_name: String,
    engine_author: String,
    options: BTreeMap<String, UciOptionField>
}

impl Engine {
    pub async fn new(path: &Path, args: &[String]) -> Result<(Self, Vec<EngineError>), EngineError> {
        let mut this = Self {
            engine: RawEngine::new(path, args).await?,
            engine_name: String::new(),
            engine_author: String::new(),
            options: BTreeMap::new()
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
                UciRemark::Option { name, info } => {
                    use UciOptionField::*;
                    match info {
                        UciOptionInfo::Check { default } => {
                            self.options.insert(name, Check { value: default });
                        }
                        UciOptionInfo::Spin { default, min, max } => {
                            if min > max || default < min || default > max {
                                Err(EngineError::InvalidOption)?;
                            }
                            self.options.insert(name, Spin { value: default, min, max });
                        }
                        UciOptionInfo::Combo { default, labels } => {
                            let value = labels.iter()
                                .position(|l| l == &default)
                                .ok_or(EngineError::InvalidOption)?;
                            self.options.insert(name, Combo { value, labels });
                        }
                        UciOptionInfo::Button => {}, //TODO
                        UciOptionInfo::String { default } => {
                            self.options.insert(name, String { value: default });
                        }
                    }
                }
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

    pub fn options(&self) -> &BTreeMap<String, UciOptionField> {
        &self.options
    }

    pub async fn set_option(&mut self, name: String, value: UciOptionValue) -> Result<(), SetOptionError> {
        let fmt_opts = self.uci_format_opts();
        let field = self.options.get_mut(&name).ok_or(SetOptionError::NoSuchOption)?;
        let opt = |value| UciCommand::SetOption { name, value: Some(value) };
        match (field, value) {
            (UciOptionField::Check { value }, UciOptionValue::Check(new)) => {
                self.engine.send(&opt(format!("{}", new)), &fmt_opts).await?;
                *value = new;
            }
            (UciOptionField::Spin { value, min, max }, UciOptionValue::Spin(new)) => {
                if new < *min || new > *max {
                    Err(SetOptionError::OutOfRange)?;
                }
                self.engine.send(&opt(format!("{}", new)), &fmt_opts).await?;
                *value = new;
            }
            (UciOptionField::Combo { value, labels }, UciOptionValue::Combo(new)) => {
                if new >= labels.len() {
                    Err(SetOptionError::OutOfRange)?;
                }
                self.engine.send(&opt(labels[new].clone()), &fmt_opts).await?;
                *value = new;
            }
            (UciOptionField::String { value }, UciOptionValue::String(new)) => {
                self.engine.send(&opt(new.clone()), &fmt_opts).await?;
                *value = new;
            }
            _ => Err(SetOptionError::TypeMismatch)?
        }
        Ok(())
    }

    pub fn chess960_supported(&self) -> bool {
        matches!(self.options.get("UCI_Chess960"), Some(&UciOptionField::Check { .. }))
    }

    pub fn chess960_enabled(&self) -> bool {
        matches!(self.options.get("UCI_Chess960"), Some(&UciOptionField::Check { value: true }))
    }

    fn uci_format_opts(&self) -> UciFormatOptions {
        UciFormatOptions {
            chess960: self.chess960_enabled(),
            wdl: false
        }
    }

    async fn send(&mut self, cmd: &UciCommand) -> Result<(), EngineError> {
        self.engine.send(cmd, &self.uci_format_opts()).await
    }

    async fn recv(&mut self) -> Result<Option<UciRemark>, EngineError> {
        self.engine.recv(&self.uci_format_opts()).await
    }

    pub fn analyze(&mut self, game: &ChessGame, limit: AnalysisLimit) -> Result<EngineAnalysis<'_>, EngineAnalysisError> {
        let chess960 = self.chess960_enabled();
        if game.needs_chess960() && !chess960 {
            Err(EngineAnalysisError::Requires960)?;
        }
        let board = game.board().clone();
        let position_cmd = game_to_position_message(game, chess960);
        let go_cmd = analysis_limit_to_go_message(limit);
        let stream = Box::pin(async_stream::try_stream! {
            self.send(&position_cmd).await?;
            self.send(&go_cmd).await?;
            loop {
                match self.recv().await?.ok_or(EngineError::UnexpectedTermination)? {
                    UciRemark::Info(info) => {
                        yield EngineAnalysisEvent::Info(info);
                    }
                    UciRemark::BestMove { mv, .. } => {
                        let mv = canonicalize_move(&board, mv, false);
                        yield EngineAnalysisEvent::BestMove(mv);
                        break;
                    }
                    rmk => {
                        yield EngineAnalysisEvent::EngineError(EngineError::UnexpectedRemark(rmk));
                    }
                }
            }
        });
        Ok(EngineAnalysis { stream })
    }
}
