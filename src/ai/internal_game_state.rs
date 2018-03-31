use ::{BOARD_HEIGHT, BOARD_WIDTH, GameState, Move, Tile};
use ai::bitboard::{BB_INVALID, BB_TARGET, Bitboard, BitIndex, pos_to_index, index_to_pos};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct InternalGameState {
    pieces: [Bitboard; 2],
    pub ply: usize,
    pub current_player: u8,
}

impl InternalGameState {
    fn is_valid_location(index: BitIndex) -> bool {
        !BB_INVALID.get_bit(index)
    }

    fn empty_bb(&self) -> Bitboard {
        !self.occupied_bb()
    }

    fn occupied_bb(&self) -> Bitboard {
        self.pieces[0] | self.pieces[1]
    }

    pub fn won(&self, player: u8) -> bool {
        self.pieces[player as usize] == BB_TARGET[player as usize]
    }

    fn moves_from(&self, from: BitIndex) -> Vec<InternalMove> {
        let mut result = Bitboard::default();
        let mut jumping_targets = Bitboard::bit(from);

        let skip_bb = !BB_INVALID & self.occupied_bb();
        let empty_bb = !BB_INVALID & self.empty_bb();

        while let Some(reachable) = jumping_targets.pop() {
            for &(skip, jump) in &[
                (255, 254), // west
                (  1,   2), // east
                ( 13,  26), // south west
                ( 14,  28), // south east
                (243, 230), // north east
                (242, 228), // north west
            ] {
                let skip = reachable.wrapping_add(skip);
                let jump = reachable.wrapping_add(jump);

                let jump_bb = empty_bb & !jumping_targets & !result;

                if skip_bb.get_bit(skip) && jump_bb.get_bit(jump) {
                    jumping_targets.set_bit(jump);
                    result.set_bit(jump);
                }
            }
        }

        for &slide in &[
            255,
              1,
             13,
             14,
            243,
            242,
        ] {
            let to = from.wrapping_add(slide);
            if empty_bb.get_bit(to) {
                result.set_bit(to);
            }
        }

        assert!(result.ones().all(|to| from != to));
        result.ones().map(|to| InternalMove { from, to }).collect()
    }

    pub fn possible_moves(&self) -> Vec<InternalMove> {
        let board = self.pieces[self.current_player as usize];

        board.ones().flat_map(|i| self.moves_from(i)).collect()
    }

    pub fn move_piece(&mut self, mov: InternalMove) {
        let from = mov.from;
        let to = mov.to;

        if !InternalGameState::is_valid_location(from) || !InternalGameState::is_valid_location(to) {
            panic!("Invalid locations for move_piece: {:X}, {:X}", from, to); 
        }

        let mut bits_to_invert = Bitboard::default();
        bits_to_invert.set_bit(from);
        bits_to_invert.set_bit(to);
        if self.pieces[0].get_bit(from) {
            self.pieces[0] = self.pieces[0] ^ bits_to_invert;
        } else if self.pieces[1].get_bit(from) {
            self.pieces[1] = self.pieces[1] ^ bits_to_invert;
        }
        self.current_player = 1-self.current_player;
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct InternalMove {
    pub from: BitIndex,
    pub to: BitIndex,
}

impl InternalMove {
    pub fn inverse(&self) -> Self {
        InternalMove {
            from: self.to,
            to: self.from,
        }
    }

    pub fn to_move(&self) -> Move {
        Move {
            from: index_to_pos(self.from),
            to: index_to_pos(self.to),
        }
    }
}

impl From<Move> for InternalMove {
    fn from(mov: Move) -> InternalMove {
        let from = pos_to_index(mov.from.0 as u8, mov.from.1 as u8);
        let to = pos_to_index(mov.to.0 as u8, mov.to.1 as u8);
        InternalMove {
            from,
            to,
        }
    }
}

impl From<GameState> for InternalGameState {
    fn from(state: GameState) -> Self {
        let mut pieces: [Bitboard; 2] = Default::default();

        for x in 0..BOARD_WIDTH {
            for y in 0..BOARD_HEIGHT {
                if let Tile::Player(player) = state.get(x as i8, y as i8) {
                    if BB_INVALID.get_bit(pos_to_index(x, y)) {
                        println!("{} {}: {2} hex: {2:X}", x, y, pos_to_index(x, y));
                    }
                    pieces[player as usize].set_bit(pos_to_index(x, y));
                }
            }
        }

        assert!((pieces[0] & BB_INVALID).is_empty());
        assert!((pieces[1] & BB_INVALID).is_empty());

        InternalGameState {
            pieces,
            ply: state.ply,
            current_player: state.current_player,
        }
    }
}

