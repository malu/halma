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

pub struct TranspositionTable {
    table: Vec<Option<(InternalGameState, Transposition)>>,
    bitmask: usize,
}

impl TranspositionTable {
    pub fn new(bits: usize) -> Self {
        let size = 1 << (bits - 1);
        let mut table = Vec::with_capacity(size);

        for _ in 0..size {
            table.push(None);
        }

        TranspositionTable {
            table,
            bitmask: size - 1,
        }
    }

    pub fn get(&self, hash: IncrementalHash, state: InternalGameState) -> Option<Transposition> {
        if let Some((tstate, t)) = self.table[hash & self.bitmask] {
            if tstate != state {
                return None;
            } else {
                return Some(t);
            }
        }
        
        None
    }

    pub fn insert(&mut self, hash: IncrementalHash, state: InternalGameState, transposition: Transposition) {
        self.table[hash & self.bitmask] = Some((state, transposition));
    }
}

