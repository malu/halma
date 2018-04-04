use std::cell::RefCell;
use std::rc::Rc;

use ai::Score;
use ai::incremental_hasher::IncrementalHash;
use ai::internal_game_state::{InternalGameState, InternalMove};
use ai::tt::TranspositionTable;

pub struct MovePicker {
    state: InternalGameState,
    hash: IncrementalHash,
    main_tt: Rc<RefCell<TranspositionTable>>,

    stage: MovePickerStage,
}

enum MovePickerStage {
    TTMove,
    Generate,
    All(usize, Vec<InternalMove>),
}

impl MovePicker {
    pub fn new(state: InternalGameState, hash: IncrementalHash, main_tt: Rc<RefCell<TranspositionTable>>) -> Self {
        MovePicker {
            state,
            hash,
            main_tt,
            stage: MovePickerStage::TTMove,
        }
    }
}

impl Iterator for MovePicker {
    type Item = InternalMove;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stage {
            MovePickerStage::TTMove => {
                self.stage = MovePickerStage::Generate;
                let tt_entry = self.main_tt.borrow().get(self.hash);
                if let Some(transposition) = tt_entry {
                    // Since we can get an illegal move from the transposition table (because it
                    // belongs to another position which hashed to the same index), we have to
                    // check its legallity before returning it.
                    let mov = transposition.best_move;
                    if self.state.pieces[self.state.current_player as usize].get_bit(mov.from) &&
                        self.state.reachable_from(mov.from).get_bit(mov.to) {
                        return Some(mov);
                    }
                }
                self.next()
            }

            MovePickerStage::Generate => {
                let moves = self.state.possible_moves();
                self.stage = MovePickerStage::All(0, moves);
                self.next()
            }

            MovePickerStage::All(ref mut index, ref mut moves) => {
                let num_moves = moves.len();
                if *index == num_moves {
                    return None;
                }

                if *index < 8 {
                    let mut max_i = *index;
                    let mut max_move_score = move_score(self.state.current_player, moves[max_i]);

                    for j in *index+1..num_moves {
                        let move_score = move_score(self.state.current_player, moves[j]);
                        if move_score > max_move_score {
                            max_i = j;
                            max_move_score = move_score;
                        }
                    }

                    moves.swap(*index, max_i);
                }

                let mov = moves[*index];
                *index += 1;
                Some(mov)
            }
        }
    }
}

fn move_score(player: u8, mov: InternalMove) -> Score {
    if player == 0 {
        mov.to as isize - mov.from as isize
    } else {
        mov.from as isize - mov.to as isize
    }
}
