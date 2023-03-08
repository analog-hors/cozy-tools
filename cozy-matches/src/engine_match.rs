use std::time::{Duration, Instant};

use futures_core::Stream;
use cozy_chess::*;
use futures_util::StreamExt;
use thiserror::Error;

use crate::time_control::TimeControl;
use crate::engine::{Engine, EngineAnalysisEvent, AnalysisSearchLimit, AnalysisLimit, AnalysisTimeLimit, EngineError};
use crate::game::ChessGame;

#[derive(Debug, Clone)]
pub struct EngineMatchConfig {
    pub white_time_control: EngineMatchTimeConfig,
    pub black_time_control: EngineMatchTimeConfig
}

#[derive(Debug, Clone)]
pub struct EngineMatchTimeConfig {
    pub search_limit: Option<AnalysisSearchLimit>,
    pub clock: ChessClockState
}

#[derive(Debug, Clone)]
pub enum ChessClockState {
    Infinite,
    MoveTime(Duration),
    Clock(TimeControl)
}

impl ChessClockState {
    pub fn update(&mut self, elapsed: Duration) -> bool {
        match self {
            ChessClockState::Infinite => false,
            ChessClockState::MoveTime(move_time) => elapsed > *move_time,
            ChessClockState::Clock(TimeControl { time, increment }) => {
                *time = (*time + *increment).saturating_sub(elapsed);
                time.is_zero()
            }
        }
    }

    pub fn as_tc(&self) -> Option<&TimeControl> {
        if let Self::Clock(tc) = self {
            Some(tc)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct EngineMatch {
    config: EngineMatchConfig,
    game: ChessGame,
    engines: [Engine; Color::NUM],
}

#[derive(Debug)]
pub enum EngineMatchEvent {
    EngineAnalysisEvent {
        engine: Color,
        event: EngineAnalysisEvent
    },
    GameOver {
        winner: Option<Color>
    }
}

#[derive(Debug, Error)]
pub enum EngineMatchError {
    #[error("engine error")]
    EngineError(#[from] EngineError)
}

impl EngineMatch {
    pub fn new(config: EngineMatchConfig, game: ChessGame, white: Engine, black: Engine) -> Self {
        Self {
            config,
            game,
            engines: [white, black]
        }
    }

    pub fn run(mut self) -> impl Stream<Item = Result<EngineMatchEvent, EngineMatchError>> {
        async_stream::try_stream! {
            let mut white_clock = self.config.white_time_control.clock.clone();
            let mut black_clock = self.config.black_time_control.clock.clone();
            
            let mut match_result = match self.game.status() {
                GameStatus::Won => Some(Some(!self.game.board().side_to_move())),
                GameStatus::Drawn => Some(None),
                GameStatus::Ongoing => None,
            };
            while match_result.is_none() {
                let stm = self.game.board().side_to_move();

                let white_tc = white_clock.as_tc();
                let black_tc = black_clock.as_tc();
                let clock = match stm {
                    Color::White => &white_clock,
                    Color::Black => &black_clock,
                };
                let time_limit = match clock {
                    ChessClockState::Infinite => AnalysisTimeLimit::Infinite,
                    ChessClockState::MoveTime(move_time) => AnalysisTimeLimit::MoveTime(*move_time),
                    ChessClockState::Clock(_) => AnalysisTimeLimit::TimeLeft {
                        white_time: white_tc.map(|c| c.time),
                        black_time: black_tc.map(|c| c.time),
                        white_increment: white_tc.map(|c| c.increment),
                        black_increment: black_tc.map(|c| c.increment),
                        moves_to_go: None
                    }
                };
                let search_limit = match stm {
                    Color::White => self.config.white_time_control.search_limit,
                    Color::Black => self.config.black_time_control.search_limit
                };
                let limit = AnalysisLimit {
                    search_limit,
                    time_limit: Some(time_limit),
                };

                let analyis_start = Instant::now();
                let analysis = self.engines[stm as usize].analyze(&self.game, limit);
                futures_util::pin_mut!(analysis);
                let mut best_move = None;
                while let Some(event) = analysis.next().await {
                    let event = event?;
                    if let EngineAnalysisEvent::BestMove(mv) = event {
                        best_move = Some(mv);
                    }
                    yield EngineMatchEvent::EngineAnalysisEvent { engine: stm, event };
                }
                let elapsed = analyis_start.elapsed();
                let timed_out = match stm {
                    Color::White => white_clock.update(elapsed),
                    Color::Black => black_clock.update(elapsed),
                };
                let best_move = best_move.unwrap();

                self.game.play(best_move);
                match_result = match self.game.status() {
                    GameStatus::Won => Some(Some(stm)),
                    GameStatus::Drawn => Some(None),
                    GameStatus::Ongoing if timed_out => Some(Some(!stm)),
                    GameStatus::Ongoing => None,
                }
            }
            let winner = match_result.unwrap();
            
            yield EngineMatchEvent::GameOver { winner };
        }
    }
}
