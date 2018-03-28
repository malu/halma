use ::{GameState, Move};
use ai::{IncrementalHash, Score};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Evaluation {
    Exact(Score),
    LowerBound(Score),
    UpperBound(Score)
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Transposition {
    pub evaluation: Evaluation,
    pub best_move: Move,
    pub depth: isize,
    pub ply: usize,
}

const TT_BITS: usize = 20;
const TT_SIZE: usize = 1 << TT_BITS;

pub struct TranspositionTable {
    table: Vec<Option<(GameState, Transposition)>>,
    pub insertion: usize,
    pub replace: usize,
    pub update: usize,
}

impl Default for TranspositionTable {
    fn default() -> Self {
        let mut table = Vec::with_capacity(TT_SIZE);

        for _ in 0..TT_SIZE {
            table.push(None);
        }

        TranspositionTable {
            table,
            insertion: 0,
            replace: 0,
            update: 0,
        }
    }
}

impl TranspositionTable {
    pub fn len(&self) -> usize {
        self.table.iter().filter(|option| option.is_some()).count()
    }

    pub fn get(&self, hash: IncrementalHash, state: GameState) -> Option<Transposition> {
        if let Some((tstate, t)) = self.table[hash % TT_SIZE] {
            if tstate != state {
                return None;
            } else {
                return Some(t);
            }
        }
        
        None
    }

    pub fn insert(&mut self, hash: IncrementalHash, state: GameState, transposition: Transposition) {
        self.insertion += 1;
        if let Some((tstate, _)) = self.table[hash % TT_SIZE] {
            if tstate == state {
                self.update += 1;
            } else {
                self.replace += 1;
            }
        }

        self.table[hash % TT_SIZE] = Some((state, transposition));
    }
}

