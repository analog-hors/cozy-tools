use std::time::Duration;

use cozy_chess::*;
use cozy_uci::remark::UciInfo;

use super::error::EngineError;

#[derive(Debug, Clone, Copy)]
pub struct AnalysisLimit {
    pub search_limit: Option<AnalysisSearchLimit>,
    pub time_limit: Option<AnalysisTimeLimit>
}

#[derive(Debug, Clone, Copy)]
pub struct AnalysisSearchLimit {
    pub nodes: Option<u64>,
    pub depth: Option<u32>
}

#[derive(Debug, Clone, Copy)]
pub enum AnalysisTimeLimit {
    Infinite,
    MoveTime(Duration),
    TimeLeft {
        white_time: Option<Duration>,
        black_time: Option<Duration>,
        white_increment: Option<Duration>,
        black_increment: Option<Duration>,
        moves_to_go: Option<u8>
    }
}

#[derive(Debug, Clone, Copy)]
pub enum UciScore {
    Centipawn(i32),
    Mate(i8)
}

#[derive(Debug)]
pub enum EngineAnalysisEvent {
    Info(UciInfo),
    BestMove(Move),
    EngineError(EngineError)
}
