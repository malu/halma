use ai::{IncrementalHash, Score};
use ai::internal_game_state::{InternalGameState, InternalMove};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ScoreType {
    Exact(Score),
    LowerBound(Score),
    UpperBound(Score)
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Transposition {
    pub evaluation: ScoreType,
    pub best_move: InternalMove,
    pub depth: isize,
    pub ply: usize,
}

impl Transposition {
    pub fn should_be_replaced_by(&self, other: &Self, pv: bool) -> bool {
        if pv {
            return true;
        }

        if self.ply + 6 < other.ply {
            return true;
        }

        if self.depth <= other.depth {
            return true;
        }

        false
    }
}

const TT_BITS: usize = 20;
const TT_SIZE: usize = 1 << TT_BITS;

pub struct TranspositionTable {
    table: Vec<Option<(InternalGameState, Transposition)>>,
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

    pub fn get(&self, hash: IncrementalHash, state: InternalGameState) -> Option<Transposition> {
        if let Some((tstate, t)) = self.table[hash % TT_SIZE] {
            if tstate != state {
                return None;
            } else {
                return Some(t);
            }
        }
        
        None
    }

    pub fn insert(&mut self, hash: IncrementalHash, state: InternalGameState, transposition: Transposition) {
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

