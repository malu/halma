use ::{BOARD_HEIGHT, BOARD_WIDTH, Move};

pub type IncrementalHash = usize;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct IncrementalHasher {
    tile_hashes: [[(IncrementalHash, IncrementalHash); BOARD_HEIGHT as usize]; BOARD_WIDTH as usize],
    to_move_hash: IncrementalHash,
}

impl Default for IncrementalHasher {
    fn default() -> Self {
        use rand::Rng;
        let mut rng = ::rand::thread_rng();

        let mut tile_hashes = [[(0, 0); BOARD_HEIGHT as usize]; BOARD_WIDTH as usize];

        for x in 0..BOARD_WIDTH as usize {
            for y in 0..BOARD_HEIGHT as usize {
                tile_hashes[x][y] = (rng.gen(), rng.gen());
            }
        }

        let to_move_hash = rng.gen();

        IncrementalHasher {
            tile_hashes,
            to_move_hash,
        }
    }
}

impl IncrementalHasher {
    pub fn update(&self, current_player: u8, mov: Move) -> IncrementalHash {
        let from;
        let to;
        if current_player == 0 {
            from = self.tile_hashes[mov.from.0 as usize][mov.from.1 as usize].0;
            to = self.tile_hashes[mov.to.0 as usize][mov.to.1 as usize].0;
        } else {
            from = self.tile_hashes[mov.from.0 as usize][mov.from.1 as usize].1;
            to = self.tile_hashes[mov.to.0 as usize][mov.to.1 as usize].1;
        }

        from ^ to ^ self.to_move_hash
    }
}
