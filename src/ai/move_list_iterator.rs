use ::Move;
use ai::Score;

pub trait MoveList {
    fn order<F>(self, n: usize, score_fn: F) -> MoveListIterator<F> where F: Fn(Move) -> Score;
}

impl MoveList for Vec<Move> {
    fn order<F>(self, n: usize, score_fn: F) -> MoveListIterator<F> where F: Fn(Move) -> Score {
        MoveListIterator {
            moves: self,
            index: 0,
            n,
            score_fn,
        }
    }
}

pub struct MoveListIterator<F> {
    moves: Vec<Move>,
    n: usize,
    index: usize,
    score_fn: F,
}

impl<F> Iterator for MoveListIterator<F> where F: Fn(Move) -> Score {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.moves.len() {
            return None;
        }

        if self.index < self.n {
            let num_moves = self.moves.len();
            let mut max_i = self.index;
            let mut max_move_score = (self.score_fn)(self.moves[max_i]);

            for j in self.index+1..num_moves {
                let move_score = (self.score_fn)(self.moves[j]);
                if move_score > max_move_score {
                    max_i = j;
                    max_move_score = move_score;
                }
            }

            self.moves.swap(self.index, max_i);
        }

        let mov = self.moves[self.index];
        self.index += 1;
        Some(mov)
    }
}

