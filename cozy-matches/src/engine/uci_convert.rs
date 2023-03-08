use cozy_chess::*;
use vampirc_uci::*;

use crate::game::ChessGame;

use super::UciOptionValue;
use super::analysis::AnalysisLimit;

pub trait UciInto<T> {
    fn uci_into(self) -> T;
}

pub trait UciMoveInto<T> {
    fn uci_move_into(self, board: &Board, chess960: bool) -> T;
}

impl UciInto<UciSquare> for Square {
    fn uci_into(self) -> UciSquare {
        UciSquare {
            file: self.file().into(),
            rank: self.rank() as u8 + 1
        }
    }
}

impl UciInto<UciPiece> for Piece {
    fn uci_into(self) -> UciPiece {
        match self {
            Self::Pawn => UciPiece::Pawn,
            Self::Knight => UciPiece::Knight,
            Self::Bishop => UciPiece::Bishop,
            Self::Rook => UciPiece::Rook,
            Self::Queen => UciPiece::Queen,
            Self::King => UciPiece::King
        }
    }
}

impl UciMoveInto<UciMove> for Move {
    fn uci_move_into(mut self, board: &Board, chess960: bool) -> UciMove {
        if !chess960 && board.color_on(self.from) == board.color_on(self.to) {
            let rights = board.castle_rights(board.side_to_move());
            let file = if Some(self.to.file()) == rights.short {
                File::G
            } else {
                File::C
            };
            self.to = Square::new(file, self.to.rank());
        }
        UciMove {
            from: self.from.uci_into(),
            to: self.to.uci_into(),
            promotion: self.promotion.map(UciInto::uci_into)
        }
    }
}

impl UciInto<Square> for UciSquare {
    fn uci_into(self) -> Square {
        Square::new(
            self.file.try_into().unwrap(),
            Rank::index(self.rank as usize - 1)
        )
    }
}

impl UciInto<Piece> for UciPiece {
    fn uci_into(self) -> Piece {
        Piece::index(self as usize)
    }
}

impl UciMoveInto<Move> for UciMove {
    fn uci_move_into(self, board: &Board, chess960: bool) -> Move {
        let mut mv = Move {
            from: self.from.uci_into(),
            to: self.to.uci_into(),
            promotion: self.promotion.map(UciInto::uci_into)
        };
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
}

pub fn game_to_position_message(game: &ChessGame) -> UciMessage {
    let fen = Some(UciFen(format!("{}", game.init_pos())));
    let mut moves = Vec::new();
    for (i, (mv, _)) in game.stack().iter().enumerate() {
        let board = if i > 0 { &game.stack()[i - 1].1 } else { game.init_pos() };
        moves.push(mv.uci_move_into(&board, false));
    }
    UciMessage::Position { startpos: false, fen, moves }
}

pub fn uci_option_to_set_option_message(name: String, value: Option<&UciOptionValue>) -> UciMessage {
    let name = name.clone();
    let value = value.as_ref().map(|v| match v {
        UciOptionValue::String(s) => s.clone(),
        UciOptionValue::Integer(i) => format!("{}", i),
        UciOptionValue::Boolean(b) => format!("{}", b)
    });
    UciMessage::SetOption { name, value }
}

pub fn analyis_limit_to_go_message(limit: AnalysisLimit) -> UciMessage {
    UciMessage::Go {
        time_control: limit.time_limit.map(Into::into),
        search_control: limit.search_limit.map(Into::into)
    }
}
