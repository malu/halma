extern crate rand;
extern crate sdl2;
extern crate serde_json;
extern crate serde;
#[macro_use] extern crate serde_derive;

pub mod ai;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tile {
    Empty,
    Invalid,
    Player(u8),
}

pub const BOARD_WIDTH: u8 = 13;
pub const BOARD_HEIGHT: u8 = 17;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    board: [[Tile; BOARD_HEIGHT as usize]; BOARD_WIDTH as usize],
    ply: usize,
    current_player: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Move {
    pub from: (i8, i8),
    pub to: (i8, i8),
}

impl Move {
    fn inverse(&self) -> Self {
        Move {
            from: self.to,
            to: self.from,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Game {
    state: GameState,
    moves: Vec<Move>,
}

impl GameState {
    fn is_valid_location(&self, x: i8, y: i8) -> bool {
        if y < 0 || y >= BOARD_HEIGHT as i8 {
            return false;
        }

        let (left, right) = [
            (6, 6), (6, 7), (5, 7), (5, 8), (0, 12), (1, 12), (1, 11), (2, 11), (2, 10), (2, 11), (1, 11), (1, 12), (0, 12), (5, 8), (5, 7), (6, 7), (6, 6)
        ][y as usize];

        x >= left && x <= right
    }

    fn set(&mut self, x: i8, y: i8, tile: Tile) {
        assert!(self.is_valid_location(x, y));
        self.board[x as usize][y as usize] = tile;
    }

    pub fn get(&self, x: i8, y: i8) -> Tile {
        assert!(x >= 0 && (x as u8) < BOARD_WIDTH);
        assert!(y >= 0 && (y as u8) < BOARD_HEIGHT);

        self.board[x as usize][y as usize]
    }

    pub fn targets(player: u8) -> &'static [(i8, i8)] {
        if player == 0 {
            &[
                (4, 12), (5, 12), (6, 12), (7, 12), (8, 12),
                (5, 13), (6, 13), (7, 13), (8, 13),
                (5, 14), (6, 14), (7, 14),
                (6, 15), (7, 15),
                (6, 16),
            ]
        } else if player == 1 {
            &[
                (6, 0),
                (6, 1), (7, 1),
                (5, 2), (6, 2), (7, 2),
                (5, 3), (6, 3), (7, 3), (8, 3),
                (4, 4), (5, 4), (6, 4), (7, 4), (8, 4),
            ]
        } else {
            unreachable!()
        }
    }

    pub fn won(&self, player: u8) -> bool {
        for &(x, y) in GameState::targets(player) {
            if self.get(x, y) != Tile::Player(player) {
                return false;
            }
        }

        return true;
    }

    pub fn moves_from(&self, x: i8, y: i8) -> Vec<Move> {
        let mut result = Vec::with_capacity(128);
        let mut jumping_targets = Vec::with_capacity(128);
        jumping_targets.push((x, y));

        while let Some((sx, sy)) = jumping_targets.pop() {
            for &(dx, dy, jx, jy) in &[(-1, 0, -2, 0), (1, 0, 2, 0), (-y%2+1, 1, 1, 2), (-y%2, 1, -1, 2), (-y%2+1, -1, 1, -2), (-y%2, -1, -1, -2)] {
                if !self.is_valid_location(sx+jx, sy+jy) {
                    continue;
                }

                if let Tile::Player(_) = self.get(sx+dx, sy+dy) {
                    if self.get(sx+jx, sy+jy) == Tile::Empty
                        && !jumping_targets.contains(&(sx+jx, sy+jy))
                        && !result.contains(&Move { from: (x, y), to: (sx+jx, sy+jy)}) {
                        jumping_targets.push((sx+jx, sy+jy));
                        result.push(Move { from: (x, y), to: (sx+jx, sy+jy) });
                    }
                }
            }
        }

        for &(dx, dy) in &[(-1, 0), (1, 0), (-y%2+1, 1), (-y%2, 1), (-y%2+1, -1), (-y%2, -1)] {
            if !self.is_valid_location(x+dx, y+dy) {
                continue;
            }

            if self.get(x+dx, y+dy) == Tile::Empty {
                result.push(Move { from: (x, y), to: (x+dx, y+dy) });
            }
        }

        assert!(result.iter().all(|&Move { from, to }| from != to));
        result
    }

    pub fn possible_moves(&self) -> Vec<Move> {
        let mut result = Vec::new();

        for x in 0..BOARD_WIDTH as i8 {
            for y in 0..BOARD_HEIGHT as i8 {
                if self.get(x, y) == Tile::Player(self.current_player) {
                    result.append(&mut self.moves_from(x, y));
                }
            }
        }
        result
    }

    fn move_piece(&mut self, mov: Move) {
        let (fx, fy) = mov.from;
        let (tx, ty) = mov.to;

        if !self.is_valid_location(fx, fy) || !self.is_valid_location(tx, ty) {
            panic!("Invalid locations for move_piece");
        }

        let from = self.get(fx, fy);
        self.set(tx, ty, from);
        self.set(fx, fy, Tile::Empty);
        self.current_player = 1-self.current_player;
    }

    pub fn current_player(&self) -> u8 {
        self.current_player
    }
}

impl Game {
    pub fn move_piece(&mut self, mov: Move) {
        let (fx, fy) = mov.from;
        let (tx, ty) = mov.to;

        if !self.state.is_valid_location(fx, fy) || !self.state.is_valid_location(tx, ty) {
            panic!("Invalid locations for move_piece");
        }

        self.moves.push(mov);
        self.state.move_piece(mov);
    }

    pub fn undo(&mut self) {
        if let Some(mov) = self.moves.pop() {
            let (fx, fy) = mov.from;
            let (tx, ty) = mov.to;
            let from = self.state.get(tx, ty);
            self.state.set(fx, fy, from);
            self.state.set(tx, ty, Tile::Empty);
            self.state.current_player = 1-self.state.current_player;
        }
    }

    pub fn last_move(&self) -> Option<&Move> {
        self.moves.last()
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }
}

impl Default for GameState {
    fn default() -> Self {
        let mut state = GameState {
            board: [[Tile::Invalid; BOARD_HEIGHT as usize]; BOARD_WIDTH as usize],
            ply: 0,
            current_player: 0,
        };

        for y in 0..BOARD_HEIGHT as i8 {
            for x in 0..BOARD_WIDTH as i8 {
                if !state.is_valid_location(x, y) {
                    continue;
                }
                if y < 5 && (x-6).abs() < 3 {
                    state.set(x, y, Tile::Player(0));
                } else if y > 11  && (x-6).abs() < 3 {
                    state.set(x, y, Tile::Player(1));
                } else {
                    state.set(x, y, Tile::Empty);
                }
            }
        }

        state
    }
}
