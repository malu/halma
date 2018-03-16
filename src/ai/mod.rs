use rayon::prelude::*;

use {BOARD_HEIGHT, BOARD_WIDTH, GameState, Move, Tile};

pub fn possible_moves(state: &GameState) -> Vec<Move> {
    let mut result = Vec::new();

    for x in 0..BOARD_WIDTH as i8 {
        for y in 0..BOARD_HEIGHT as i8 {
            if state.get(x, y) == Tile::Player(state.current_player) {
                result.extend(state.reachable_from(x, y).into_iter().map(|to| Move { from: (x, y), to } ));
            }
        }
    }

    result
}


pub struct AI {
    state: GameState,
}

impl AI {
    pub fn new(state: GameState) -> AI {
        AI {
            state
        }
    }

    fn evaluate_position(&self, depth: usize) -> i64 {
        if depth == 0 {
            let mut score = 0.0;
            let score_dist_last_piece = {
                let mut p1_dist = 0;
                let mut p2_dist = 0;

                for x in 0..BOARD_WIDTH as i8 {
                    for y in 0..BOARD_HEIGHT as i8 {
                        if self.state.get(x, y) == Tile::Player(1) {
                            p1_dist = ::std::cmp::max(p1_dist, BOARD_HEIGHT as i8 -1-y);
                        } else if self.state.get(x, y) == Tile::Player(2) {
                            p2_dist = ::std::cmp::max(p2_dist, y);
                        }
                    }
                }

                (p2_dist-p1_dist) as f32
            };
            score += score_dist_last_piece;

            let score_dist_avg_piece = {
                let mut p1_total_dist: i64 = 0;
                let mut p2_total_dist: i64 = 0;

                for x in 0..BOARD_WIDTH as i8 {
                    for y in 0..BOARD_HEIGHT as i8 {
                        if self.state.get(x, y) == Tile::Player(1) {
                            p1_total_dist += BOARD_HEIGHT as i64 - 1 - y as i64;
                        } else if self.state.get(x, y) == Tile::Player(2) {
                            p2_total_dist += y as i64;
                        }
                    }
                }

                (p2_total_dist-p1_total_dist) as f32 / 15.0
            };
            score += score_dist_avg_piece;

            (score*1_000_000.0) as i64
        } else {
            let moves = possible_moves(&self.state);
            let scores = moves.into_iter().map(|mov| {
                let mut state = self.state;
                state.move_piece(mov);
                AI::new(state).evaluate_position(depth-1)
            });
            if self.state.current_player == 1 {
                scores.max().unwrap()
            } else {
                scores.min().unwrap()
            }
        }
    }

    pub fn calculate_move(&self, depth: usize) -> Move {
        let moves = possible_moves(&self.state);
        println!("#moves: {}", moves.len());
        println!("depth: {}", depth);
        let start = ::std::time::Instant::now();
        let mov = if self.state.current_player == 1 {
            moves.into_par_iter().max_by_key(|&mov| {
                let mut state = self.state;
                state.move_piece(mov);
                AI::new(state).evaluate_position(depth)
            }).unwrap()
        } else {
            moves.into_par_iter().min_by_key(|&mov| {
                let mut state = self.state;
                state.move_piece(mov);
                AI::new(state).evaluate_position(depth)
            }).unwrap()
        };

        let end = ::std::time::Instant::now();
        let elapsed = end-start;
        println!("Took {} s", elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0);

        mov
    }
}
