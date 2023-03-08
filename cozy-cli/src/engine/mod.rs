use std::collections::BTreeMap;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use async_stream::try_stream;
use futures_core::stream::Stream;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use vampirc_uci::{UciMessage, UciFen, UciInfoAttribute};
use cozy_chess::*;

use crate::game::ChessGame;

mod uci_convert;

use uci_convert::*;

struct RawEngine {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
}

impl RawEngine {
    pub async fn new(path: &Path, args: &[String]) -> tokio::io::Result<Self> {
        let mut child = Command::new(path)
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(args)
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stderr = BufReader::new(child.stderr.take().unwrap());

        Ok(Self {
            child,
            stdin,
            stdout,
            stderr,
        })
    }

    pub async fn send(&mut self, message: UciMessage) -> tokio::io::Result<()> {
        let message = format!("{}\n", message);
        self.stdin.write_all(message.as_bytes()).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> tokio::io::Result<Option<UciMessage>> {
        let mut message = String::new();
        if self.stdout.read_line(&mut message).await? == 0 {
            return Ok(None);
        }
        let message = vampirc_uci::parse_one(&message);
        Ok(Some(message))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    path: String,
    args: Vec<String>,
    options: BTreeMap<String, Option<UciOptionValue>>,
    #[serde(default)]
    allow_invalid_options: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum UciOptionValue {
    String(String),
    Integer(i64),
    Boolean(bool),
}

pub struct Engine {
    config: EngineConfig,
    engine: RawEngine,
}

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("io error")]
    IoError(#[from] tokio::io::Error),
    #[error("engine unexpectedly exited")]
    UnexpectedTermination
}

fn game_to_uci_position(game: &ChessGame) -> UciMessage {
    let fen = Some(UciFen(format!("{}", game.init_pos())));
    let mut moves = Vec::new();
    for (i, (mv, _)) in game.stack().iter().enumerate() {
        let board = if i > 0 { &game.stack()[i - 1].1 } else { game.init_pos() };
        moves.push(mv.uci_move_into(&board, false));
    }
    UciMessage::Position { startpos: false, fen, moves }
}

#[derive(Debug, Clone, Copy)]
pub enum EngineScore {
    Centipawn(i32),
    Mate(i8)
}

#[derive(Debug, Clone, Copy)]
pub enum AnalysisLimit {
    
}

#[derive(Debug, Default, Clone)]
pub struct AnalysisInfo {
    depth: Option<u32>,
    seldepth: Option<u32>,
    time: Option<Duration>,
    nodes: Option<u64>,
    pv: Option<Vec<Move>>,
    score: Option<EngineScore>,
    hashfull: Option<u16>,
    nps: Option<u64>,
    tbhits: Option<u64>
}

impl AnalysisInfo {
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
                        info.score = Some(EngineScore::Centipawn(cp));
                    }
                    if let Some(moves) = mate {
                        info.score = Some(EngineScore::Mate(moves));
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
pub enum EngineAnalysis {
    Info(AnalysisInfo),
    BestMove(Move)
}

impl Engine {
    pub async fn new(config: EngineConfig) -> Result<Self, EngineError> {
        let mut engine = RawEngine::new(Path::new(&config.path), &config.args).await?;

        engine.send(UciMessage::Uci).await?;
        loop {
            match engine.recv().await? {
                Some(UciMessage::UciOk) => break,
                None => return Err(EngineError::UnexpectedTermination),
                //TODO handle
                _ => {}
            }
        }

        for (name, value) in &config.options {
            let name = name.clone();
            let value = value.as_ref().map(|v| match v {
                UciOptionValue::String(s) => s.clone(),
                UciOptionValue::Integer(i) => format!("{}", i),
                UciOptionValue::Boolean(b) => format!("{}", b)
            });
            engine.send(UciMessage::SetOption { name, value }).await?;
        }

        Ok(Self {
            config,
            engine
        })
    }

    pub fn analyze(&mut self, game: &ChessGame) -> impl Stream<Item = Result<EngineAnalysis, EngineError>> + '_ {
        let board = game.board().clone();
        let uci_position = game_to_uci_position(game);
        try_stream! {
            self.engine.send(uci_position).await?;
            //TODO
            self.engine.send(UciMessage::go_movetime(vampirc_uci::Duration::milliseconds(5000))).await?;
            loop {
                match self.engine.recv().await? {
                    Some(UciMessage::Info(info)) => {
                        let info = AnalysisInfo::from_uci(info, &board, false);
                        yield EngineAnalysis::Info(info);
                    }
                    Some(UciMessage::BestMove { best_move, .. }) => {
                        let best_move = best_move.uci_move_into(&board, false);
                        yield EngineAnalysis::BestMove(best_move);
                        break;
                    }
                    None => Err(EngineError::UnexpectedTermination)?,
                    // TODO handle
                    _ => {}
                }
            }
        }
    }
}
