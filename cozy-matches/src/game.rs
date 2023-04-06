use cozy_chess::*;

#[derive(Debug, Clone)]
pub struct ChessGame {
    init_pos: Board,
    stack: Vec<(Move, Board)>
}

impl ChessGame {
    pub fn new(init_pos: Board) -> Self {
        Self {
            init_pos,
            stack: Vec::new()
        }
    }

    pub fn init_pos(&self) -> &Board {
        &self.init_pos
    }

    pub fn stack(&self) -> &[(Move, Board)] {
        &self.stack
    }

    pub fn board(&self) -> &Board {
        self.stack.last().map_or(&self.init_pos, |(_, b)| b)
    }

    pub fn needs_chess960(&self) -> bool {
        let standard = |color| {
            let rights = self.init_pos.castle_rights(color);
            matches!(
                (rights.long, rights.short),
                (None | Some(File::A), None | Some(File::H))
            )
        };
        !standard(Color::White) || !standard(Color::Black)
    }

    pub fn status(&self) -> GameStatus {
        let status = self.board().status();
        if status != GameStatus::Ongoing {
            return status;
        }
        let repetitions = self.stack.iter()
            .filter(|(_, b)| b.same_position(self.board()))
            .count();
        if repetitions >= 3 {
            return GameStatus::Drawn;
        }
        GameStatus::Ongoing
    }

    pub fn play(&mut self, mv: Move) {
        let mut child = self.board().clone();
        child.play(mv);
        self.stack.push((mv, child));
    }
}
