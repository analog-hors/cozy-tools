use cozy_chess::*;
use cozy_uci::command::{UciCommand, UciInitPos, UciGoParams};

use crate::game::ChessGame;

use super::analysis::AnalysisLimit;

pub fn decanonicalize_move(board: &Board, mut mv: Move, chess960: bool) -> Move {
    if !chess960 && board.color_on(mv.from) == board.color_on(mv.to) {
        let rights = board.castle_rights(board.side_to_move());
        let file = if Some(mv.to.file()) == rights.short {
            File::G
        } else {
            File::C
        };
        mv.to = Square::new(file, mv.to.rank());
    }
    mv
}

pub fn canonicalize_move(board: &Board, mut mv: Move, chess960: bool) -> Move {
    let convert_castle = !chess960
        && board.piece_on(mv.from) == Some(Piece::King)
        && mv.from.file() == File::E
        && matches!(mv.to.file(), File::C | File::G);
    if convert_castle {
        let file = if mv.to.file() == File::C {
            File::A
        } else {
            File::H
        };
        mv.to = Square::new(file, mv.to.rank());
    }
    mv
}

pub fn game_to_position_message(game: &ChessGame, chess960: bool) -> UciCommand {
    let init_pos = UciInitPos::Board(game.init_pos().clone());
    let mut moves = Vec::new();
    for (i, (mv, _)) in game.stack().iter().enumerate() {
        let board = if i > 0 { &game.stack()[i - 1].1 } else { game.init_pos() };
        moves.push(decanonicalize_move(board, *mv, false));
    }
    UciCommand::Position { init_pos, moves }
}

pub fn analysis_limit_to_go_message(limit: AnalysisLimit) -> UciCommand {
    let mut params = UciGoParams::default();
    if let Some(search_limit) = &limit.search_limit {
        params.depth = search_limit.depth;
        params.nodes = search_limit.nodes;
    }
    UciCommand::Go(params)
}
