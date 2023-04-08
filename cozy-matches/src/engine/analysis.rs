use std::time::Duration;
use std::pin::Pin;

use tokio_stream::Stream;
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
        moves_to_go: Option<u32>
    }
}

#[derive(Debug)]
pub enum EngineAnalysisEvent {
    Info(UciInfo),
    BestMove(Move),
    EngineError(EngineError)
}

pub struct EngineAnalysis<'s> {
    pub(super) stream: Pin<Box<dyn Stream<Item = Result<EngineAnalysisEvent, EngineError>> + 's>>
}

impl<'s> Stream for EngineAnalysis<'s> {
    type Item = Result<EngineAnalysisEvent, EngineError>;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx)
    }
}
