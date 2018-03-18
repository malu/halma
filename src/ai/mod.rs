use std::collections::HashMap;

use {BOARD_HEIGHT, BOARD_WIDTH, GameState, Move, Tile};

pub struct AI {
    pub state: GameState,
    transpositions: HashMap<GameState, Transposition>,
    visited_nodes: usize,
    visited_leaf_nodes: usize,
    alpha_cutoffs: usize,
    beta_cutoffs: usize,
    tt_hits: usize,
    tt_cutoffs: usize,
    moves_explored: [usize; 8],
}

impl AI {
    pub fn new(state: GameState) -> AI {
        AI {
            state,
            transpositions: HashMap::new(),
            visited_nodes: 0,
            visited_leaf_nodes: 0,
            alpha_cutoffs: 0,
            beta_cutoffs: 0,
            tt_hits: 0,
            tt_cutoffs: 0,
            moves_explored: [0; 8],
        }
    }

    pub fn possible_moves(&self) -> Vec<Move> {
        let mut result = Vec::new();

        for x in 0..BOARD_WIDTH as i8 {
            for y in 0..BOARD_HEIGHT as i8 {
                if self.state.get(x, y) == Tile::Player(self.state.current_player) {
                    result.extend(self.state.reachable_from(x, y).into_iter().map(|to| Move { from: (x, y), to } ));
                }
            }
        }

        if self.state.current_player == 1 {
            result.sort_by_key(|&Move { from: (_, y), to: (_, y2) }| y-y2);
        } else {
            result.sort_by_key(|&Move { from: (_, y), to: (_, y2) }| y2-y);
        }
        result
    }

    fn search_max(&mut self, alpha: i64, beta: i64, depth: usize) -> i64 {
        self.visited_nodes += 1;
        let mut moves_explored = 0;
        let mut alpha = alpha;
        let mut beta = beta;

        let mut best_tt_move = None;

        if let Some(transposition) = self.transpositions.get(&self.state) {
            self.tt_hits += 1;
            match transposition.evaluation {
                Evaluation::UpperBound(upper_bound) => {
                    if upper_bound <= alpha {
                        self.tt_cutoffs += 1;
                        return upper_bound;
                    }
                    alpha = upper_bound;
                }
                Evaluation::LowerBound(lower_bound) => {
                    if lower_bound >= beta {
                        self.tt_cutoffs += 1;
                        return lower_bound;
                    }
                    beta = lower_bound;
                }
                Evaluation::Exact(_) => {}
            }

            best_tt_move = Some(transposition.best_move);
        }

        if depth == 0 {
            return self.evaluate_position();
        }

        let mut is_exact = false;
        let moves = self.possible_moves();
        let mut best_move = moves[0];

        if let Some(mov) = best_tt_move {
            self.state.move_piece(mov);
            let best_tt_move_score = self.search_min(alpha, beta, depth-1);
            self.state.move_piece(mov.inverse());
            moves_explored += 1;

            if best_tt_move_score >= beta {
                self.tt_cutoffs += 1;
                self.transpositions.insert(self.state, Transposition { evaluation: Evaluation::LowerBound(beta), best_move: mov });
                self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
                return beta;
            }

            if best_tt_move_score > alpha {
                is_exact = true;
                best_move = mov;
                alpha = best_tt_move_score;
            }
        }

        for mov in moves {
            self.state.move_piece(mov);
            let score = self.search_min(alpha, beta, depth-1);
            self.state.move_piece(mov.inverse());
            moves_explored += 1;

            if score >= beta {
                self.beta_cutoffs += 1;
                self.transpositions.insert(self.state, Transposition { evaluation: Evaluation::LowerBound(beta), best_move: mov });
                self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
                return beta;
            }

            if score > alpha {
                is_exact = true;
                best_move = mov;
                alpha = score;
            }
        }

        if is_exact {
            self.transpositions.insert(self.state, Transposition { evaluation: Evaluation::Exact(alpha), best_move });
        } else {
            self.transpositions.insert(self.state, Transposition { evaluation: Evaluation::UpperBound(alpha), best_move });
        }

        self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
        alpha
    }

    fn search_min(&mut self, alpha: i64, beta: i64, depth: usize) -> i64 {
        self.visited_nodes += 1;
        let mut moves_explored = 0;
        let mut alpha = alpha;
        let mut beta = beta;
        let mut best_tt_move = None;

        if let Some(transposition) = self.transpositions.get(&self.state) {
            self.tt_hits += 1;
            match transposition.evaluation {
                Evaluation::UpperBound(upper_bound) => {
                    if beta <= upper_bound {
                        self.tt_cutoffs += 1;
                        self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
                        return upper_bound;
                    }
                    beta = upper_bound;
                }
                Evaluation::LowerBound(lower_bound) => {
                    if alpha >= lower_bound {
                        self.tt_cutoffs += 1;
                        self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
                        return lower_bound;
                    }
                    alpha = lower_bound;
                }
                Evaluation::Exact(_) => {}
            }

            best_tt_move = Some(transposition.best_move);
        }

        if depth == 0 {
            return -self.evaluate_position();
        }

        let moves = self.possible_moves();
        let mut best_move = moves[0];
        let mut is_exact = false;

        if let Some(mov) = best_tt_move {
            self.state.move_piece(mov);
            let best_tt_move_score = self.search_max(alpha, beta, depth-1);
            self.state.move_piece(mov.inverse());
            moves_explored += 1;

            if best_tt_move_score <= alpha {
                self.tt_cutoffs += 1;
                self.transpositions.insert(self.state, Transposition { evaluation: Evaluation::UpperBound(alpha), best_move: mov });
                self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
                return alpha;
            }

            if best_tt_move_score < beta {
                is_exact = true;
                best_move = mov;
                beta = best_tt_move_score;
            }
        }

        for mov in moves {
            self.state.move_piece(mov);
            let score = self.search_max(alpha, beta, depth-1);
            self.state.move_piece(mov.inverse());
            moves_explored += 1;

            if score <= alpha {
                self.alpha_cutoffs += 1;
                self.transpositions.insert(self.state, Transposition { evaluation: Evaluation::UpperBound(alpha), best_move: mov });
                self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
                return alpha;
            }

            if score < beta {
                is_exact = true;
                best_move = mov;
                beta = score;
            }
        }

        if is_exact {
            self.transpositions.insert(self.state, Transposition { evaluation: Evaluation::Exact(beta), best_move });
        } else {
            self.transpositions.insert(self.state, Transposition { evaluation: Evaluation::LowerBound(beta), best_move });
        }

        self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
        beta
    }

    fn evaluate_position(&mut self) -> i64 {
        self.visited_leaf_nodes += 1;

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
                    let xdiff = if y % 2 == 0 {
                        (x-6).abs() as i64
                    } else {
                        ::std::cmp::min((x-6).abs(), (x-7).abs()) as i64
                    };

                    if self.state.get(x, y) == Tile::Player(1) {
                        p1_total_dist += 2*(BOARD_HEIGHT as i64 - 1 - y as i64) + xdiff;
                    } else if self.state.get(x, y) == Tile::Player(2) {
                        p2_total_dist += 2*(y as i64) + xdiff;
                    }
                }
            }

            (p2_total_dist-p1_total_dist) as f32 / 2.0 / 15.0
        };
        score += score_dist_avg_piece;

        if self.state.current_player == 1 {
            (score*1_000.0) as i64
        } else {
            (-score*1_000.0) as i64
        }
    }

    pub fn calculate_move(&mut self, depth: usize) -> Move {
        // reset statistics
        self.visited_nodes = 0;
        self.visited_leaf_nodes = 0;
        self.alpha_cutoffs = 0;
        self.beta_cutoffs = 0;
        self.tt_hits = 0;
        self.tt_cutoffs = 0;
        self.moves_explored = [0; 8];

        println!("Search depth:  {}", depth);
        let start = ::std::time::Instant::now();
        let mut moves = self.possible_moves();
        let mut score = i64::min_value();
        for d in 0..depth {
            let alpha = i64::min_value();
            let beta = i64::max_value();

            moves.sort_by_key(|&mov| {
                self.state.move_piece(mov);
                let v = self.search_min(alpha, beta, d);
                self.state.move_piece(mov.inverse());
                score = ::std::cmp::max(v, score);
                -v
            });
        }

        let end = ::std::time::Instant::now();
        let elapsed = end-start;
        let secs = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0;
        println!("  nodes | total   {} ({:.3} nodes/s)", self.visited_nodes, self.visited_nodes as f64 / secs);
        println!("        | leaf    {} ({:.2}%)", self.visited_leaf_nodes, 100.0 * self.visited_leaf_nodes as f64 / self.visited_nodes as f64);
        println!("cutoffs | alpha   {} ({:.2}%)", self.alpha_cutoffs, 100.0 * self.alpha_cutoffs as f64 / self.visited_nodes as f64);
        println!("        | beta    {} ({:.2}%)", self.beta_cutoffs, 100.0 * self.beta_cutoffs as f64 / self.visited_nodes as f64);
        let tt_lookups = self.visited_nodes;
        println!("     TT | lookups {}", tt_lookups);
        println!("        | hits    {} ({:.2}%)", self.tt_hits, 100.0 * self.tt_hits as f64 / tt_lookups as f64);
        println!("        | cutoffs {} ({:.2}%)", self.tt_cutoffs, 100.0 * self.tt_cutoffs as f64 / tt_lookups as f64);
        println!("        | size    {}", self.transpositions.len());
        let total_moves_explored = self.moves_explored.iter().sum::<usize>() as f64;
        println!("  expl. | 0:  {} ({:.3}%)", self.moves_explored[0], 100.0 * self.moves_explored[0] as f64 / total_moves_explored);
        println!("        | 1:  {} ({:.3}%)", self.moves_explored[1], 100.0 * self.moves_explored[1] as f64 / total_moves_explored);
        println!("        | 2:  {} ({:.3}%)", self.moves_explored[2], 100.0 * self.moves_explored[2] as f64 / total_moves_explored);
        println!("        | 3:  {} ({:.3}%)", self.moves_explored[3], 100.0 * self.moves_explored[3] as f64 / total_moves_explored);
        println!("        | 4:  {} ({:.3}%)", self.moves_explored[4], 100.0 * self.moves_explored[4] as f64 / total_moves_explored);
        println!("        | 5:  {} ({:.3}%)", self.moves_explored[5], 100.0 * self.moves_explored[5] as f64 / total_moves_explored);
        println!("        | 6:  {} ({:.3}%)", self.moves_explored[6], 100.0 * self.moves_explored[6] as f64 / total_moves_explored);
        println!("        | 7+: {} ({:.3}%)", self.moves_explored[7], 100.0 * self.moves_explored[7] as f64 / total_moves_explored);
        println!("");
        println!("Time:  {}:{}", elapsed.as_secs() / 60, (elapsed.as_secs() % 60) as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0);
        println!("Score: {}", score);
        println!("");

        moves[0]
    }
}

enum Evaluation {
    Exact(i64),
    LowerBound(i64),
    UpperBound(i64)
}

struct Transposition {
    evaluation: Evaluation,
    best_move: Move,
}

