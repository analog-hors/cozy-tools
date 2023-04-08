use std::collections::BTreeMap;

use futures_util::StreamExt;
use cozy_chess::Board;
use cozy_matches::engine_match::{EngineMatch, EngineMatchConfig, EngineMatchTimeConfig, ChessClockState, EngineMatchEvent};
use cozy_matches::game::ChessGame;
use cozy_matches::time_control::TimeControl;
use serde::{Deserialize, Serialize};
use clap::{Parser, Subcommand};

use cozy_matches::engine::{Engine, EngineAnalysisEvent};

#[derive(Debug, Serialize, Deserialize)]
struct CozyCliConfig {
    engines: BTreeMap<String, EngineConfig>,
}

fn clap_parse_time_control(s: &str) -> Result<TimeControl, String> {
    s.parse().map_err(|e| format!("{}", e))
}

#[derive(Debug, Parser)]
struct CozyCliArgs {
    #[clap(subcommand)]
    subcommand: Commands
}

#[derive(Debug, Subcommand)]
enum Commands {
    RunGame {
        #[clap(short, long)]
        white: String,
        #[clap(short, long)]
        black: String,
        #[clap(long = "tc", value_parser = clap_parse_time_control)]
        time_control: TimeControl
    }
}

#[tokio::main]
async fn main() {
    let args = CozyCliArgs::parse();
    let config = std::fs::read_to_string("cozy-cli-config.json").unwrap();
    let config: CozyCliConfig = serde_json::from_str(&config).unwrap();
    
    match args.subcommand {
        Commands::RunGame {
            white,
            black,
            time_control
        } => {
            let white_config = config.engines.get(&white).unwrap();
            let black_config = config.engines.get(&black).unwrap();

            let white_engine = Engine::new(white_config.clone()).await.unwrap().value;
            let black_engine = Engine::new(black_config.clone()).await.unwrap().value;
            
            let config = EngineMatchConfig {
                white_time_control: EngineMatchTimeConfig {
                    search_limit: None,
                    clock: ChessClockState::Clock(time_control)
                },
                black_time_control: EngineMatchTimeConfig {
                    search_limit: None,
                    clock: ChessClockState::Clock(time_control)
                },
            };
            let game = ChessGame::new(Board::default());
            let engine_match = EngineMatch::new(config, game, white_engine, black_engine).unwrap();
            let events = engine_match.run();
            futures_util::pin_mut!(events);
            while let Some(event) = events.next().await {
                let event = event.unwrap();
                match event {
                    EngineMatchEvent::EngineAnalysisEvent { engine, event } => match event {
                        EngineAnalysisEvent::Info(_) => {},
                        EngineAnalysisEvent::BestMove(mv) => println!("{engine}: {mv}"),
                        EngineAnalysisEvent::EngineError(e) => todo!("engine error: {}", e),
                    }
                    EngineMatchEvent::GameOver { winner } => println!("winner: {winner:?}"),
                }
            }
        }
    }
}
