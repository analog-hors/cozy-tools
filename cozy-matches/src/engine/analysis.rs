use std::time::Duration;

use cozy_chess::*;
use vampirc_uci::{UciInfoAttribute, UciSearchControl, UciTimeControl};

use super::uci_convert::UciMoveInto;

#[derive(Debug, Clone, Copy)]
pub struct AnalysisLimit {
    pub search_limit: Option<AnalysisSearchLimit>,
    pub time_limit: Option<AnalysisTimeLimit>
}

#[derive(Debug, Clone, Copy)]
pub struct AnalysisSearchLimit {
    pub nodes: Option<u64>,
    pub depth: Option<u8>
}

impl From<AnalysisSearchLimit> for UciSearchControl {
    fn from(limit: AnalysisSearchLimit) -> Self {
        Self {
            search_moves: Vec::new(),
            mate: None,
            depth: limit.depth,
            nodes: limit.nodes
        }
    }
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

impl From<AnalysisTimeLimit> for UciTimeControl {
    fn from(limit: AnalysisTimeLimit) -> Self {
        fn convert_time(d: Duration) -> vampirc_uci::Duration {
            vampirc_uci::Duration::from_std(d).unwrap_or(vampirc_uci::Duration::max_value())
        }
        match limit {
            AnalysisTimeLimit::Infinite => Self::Infinite,
            AnalysisTimeLimit::MoveTime(d) => Self::MoveTime(convert_time(d)),
            AnalysisTimeLimit::TimeLeft {
                white_time,
                black_time,
                white_increment,
                black_increment,
                moves_to_go
            } => Self::TimeLeft {
                white_time: white_time.map(convert_time),
                black_time: black_time.map(convert_time),
                white_increment: white_increment.map(convert_time),
                black_increment: black_increment.map(convert_time),
                moves_to_go
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum UciScore {
    Centipawn(i32),
    Mate(i8)
}

#[derive(Debug, Default, Clone)]
pub struct EngineAnalysisInfo {
    depth: Option<u32>,
    seldepth: Option<u32>,
    time: Option<Duration>,
    nodes: Option<u64>,
    pv: Option<Vec<Move>>,
    score: Option<UciScore>,
    hashfull: Option<u16>,
    nps: Option<u64>,
    tbhits: Option<u64>
}

impl EngineAnalysisInfo {
    pub fn from_uci(uci_info: Vec<UciInfoAttribute>, board: &Board, chess960: bool) -> Self {
        let mut info = Self::default();
        for uci_info in uci_info {
            match uci_info {
                UciInfoAttribute::Depth(depth) => info.depth = Some(depth as u32),
                UciInfoAttribute::SelDepth(seldepth) => info.seldepth = Some(seldepth as u32),
                UciInfoAttribute::Time(time) => info.time = Some(time.to_std().unwrap()),
                UciInfoAttribute::Nodes(nodes) => info.nodes = Some(nodes),
                UciInfoAttribute::Pv(uci_pv) => {
                    let mut pv = Vec::new();
                    let mut board = board.clone();
                    for mv in uci_pv {
                        let mv = mv.uci_move_into(&board, chess960);
                        if board.try_play(mv).is_err() {
                            //TODO handle errors
                            break;
                        }
                        pv.push(mv);
                    }
                    info.pv = Some(pv);
                }
                UciInfoAttribute::Score { cp, mate, .. } => {
                    if let Some(cp) = cp {
                        info.score = Some(UciScore::Centipawn(cp));
                    }
                    if let Some(moves) = mate {
                        info.score = Some(UciScore::Mate(moves));
                    }
                }
                UciInfoAttribute::HashFull(hashfull) => info.hashfull = Some(hashfull),
                UciInfoAttribute::Nps(nps) => info.nps = Some(nps),
                UciInfoAttribute::TbHits(tbhits) => info.tbhits = Some(tbhits),
                _ => {}
            }
        }
        info
    }
}

#[derive(Debug, Clone)]
pub enum EngineAnalysisEvent {
    Info(EngineAnalysisInfo),
    BestMove(Move),
    UnexpectedMessage(String)
}
