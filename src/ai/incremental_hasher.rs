use ai::internal_game_state::InternalMove;

pub type IncrementalHash = usize;

#[derive(Copy, Clone)]
pub struct IncrementalHasher {
    tile_hashes: [(IncrementalHash, IncrementalHash); 256],
    to_move_hash: IncrementalHash,
}

impl Default for IncrementalHasher {
    fn default() -> Self {
        use rand::Rng;
        let mut rng = ::rand::thread_rng();

        let mut tile_hashes = [(0, 0); 256];

        for i in 0..256 {
            tile_hashes[i] = (rng.gen(), rng.gen());
        }

        let to_move_hash = rng.gen();

        IncrementalHasher {
            tile_hashes,
            to_move_hash,
        }
    }
}

impl IncrementalHasher {
    pub fn update(&self, current_player: u8, mov: InternalMove) -> IncrementalHash {
        let from;
        let to;
        if current_player == 0 {
            from = self.tile_hashes[mov.from as usize].0;
            to = self.tile_hashes[mov.to as usize].0;
        } else {
            from = self.tile_hashes[mov.from as usize].1;
            to = self.tile_hashes[mov.to as usize].1;
        }

        from ^ to ^ self.to_move_hash
    }
}
