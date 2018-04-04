use ::{BOARD_HEIGHT, BOARD_WIDTH, GameState, Move, Tile};
use ai::bitboard::{BB_INVALID, BB_TARGET, Bitboard, BitIndex, pos_to_index, index_to_pos};

pub type Ply = u32;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct InternalGameState {
    pub pieces: [Bitboard; 2],
    pub ply: Ply,
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
        (self.pieces[0] | self.pieces[1]) & BB_TARGET[player as usize] == BB_TARGET[player as usize] && !(self.pieces[player as usize] & BB_TARGET[player as usize]).is_empty()
    }

    pub fn reachable_from(&self, from: BitIndex) -> Bitboard {
        let mut jumping_targets = Bitboard::default();
        let mut next_jumping_targets = Bitboard::bit(from);

        let occupied = self.occupied_bb();
        let empty = !BB_INVALID & self.empty_bb();

        while jumping_targets != next_jumping_targets {
            jumping_targets = next_jumping_targets;

            // shift left
            for &(skip, jump) in &[
                (  1,   2), // east
                ( 13,  26), // south west
                ( 14,  28), // south east
            ] {
                next_jumping_targets |= (occupied << skip) & (jumping_targets << jump);
            }

            // shift right
            for &(skip, jump) in &[
                ( 1,  2), // west
                (13, 26), // north east
                (14, 28), // north west
            ] {
                next_jumping_targets |= (occupied >> skip) & (jumping_targets >> jump);
            }

            next_jumping_targets &= empty;
        }

        for &slide in &[
            255, // west
              1, // east
             13, // south west
             14, // south east
            243, // north west
            242, // north east
        ] {
            let to = from.wrapping_add(slide);
            jumping_targets.set_bit(to);
        }

        jumping_targets & empty
    }

    pub fn possible_moves(&self) -> Vec<InternalMove> {
        let board = self.pieces[self.current_player as usize];
        let mut result = Vec::with_capacity(256);

        for from in board.ones() {
            result.extend(self.reachable_from(from).ones().map(|to| InternalMove { from, to } ));
        }

        result
    }

    pub fn make_move(&mut self, mov: InternalMove) {
        self.pieces[self.current_player as usize].set_bit(mov.to);
        self.pieces[self.current_player as usize].unset_bit(mov.from);
        self.current_player = 1-self.current_player;
    }

    pub fn unmake_move(&mut self, mov: InternalMove) {
        self.current_player = 1-self.current_player;
        self.pieces[self.current_player as usize].set_bit(mov.from);
        self.pieces[self.current_player as usize].unset_bit(mov.to);
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
                    pieces[player as usize].set_bit(pos_to_index(x, y));
                }
            }
        }

        assert!((pieces[0] & BB_INVALID).is_empty());
        assert!((pieces[1] & BB_INVALID).is_empty());

        InternalGameState {
            pieces,
            ply: state.ply as Ply,
            current_player: state.current_player,
        }
    }
}

mod tests{
    #[test]
    fn test_pos_to_index() {
        use ai::internal_game_state::pos_to_index;
        assert_eq!(pos_to_index(6, 0), 0x06);
        assert_eq!(pos_to_index(6, 1), 0x13);
        assert_eq!(pos_to_index(7, 1), 0x14);
        assert_eq!(pos_to_index(5, 2), 0x20);
        assert_eq!(pos_to_index(6, 2), 0x21);
        assert_eq!(pos_to_index(7, 2), 0x22);
        assert_eq!(pos_to_index(6, 16), 0xDE);
    }
}
