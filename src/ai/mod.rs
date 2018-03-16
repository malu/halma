use {BOARD_HEIGHT, BOARD_WIDTH, GameState, Move, Tile};

pub struct AI {
    state: GameState,
    visited_nodes: usize,
    visited_leaf_nodes: usize,
    alpha_cutoffs: usize,
    beta_cutoffs: usize,
}

impl AI {
    pub fn new(state: GameState) -> AI {
        AI {
            state,
            visited_nodes: 0,
            visited_leaf_nodes: 0,
            alpha_cutoffs: 0,
            beta_cutoffs: 0,
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
        if depth == 0 {
            return self.evaluate_position();
        }

        let mut alpha = alpha;

        for mov in self.possible_moves() {
            self.state.move_piece(mov);
            let score = self.search_min(alpha, beta, depth-1);
            self.state.move_piece(mov.inverse());

            if score >= beta {
                self.beta_cutoffs += 1;
                return beta;
            }

            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }

    fn search_min(&mut self, alpha: i64, beta: i64, depth: usize) -> i64 {
        self.visited_nodes += 1;
        if depth == 0 {
            return -self.evaluate_position();
        }

        let mut beta = beta;

        for mov in self.possible_moves() {
            self.state.move_piece(mov);
            let score = self.search_max(alpha, beta, depth-1);
            self.state.move_piece(mov.inverse());

            if score <= alpha {
                self.alpha_cutoffs += 1;
                return alpha;
            }

            if score < beta {
                beta = score;
            }
        }

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

        if self.state.current_player == 1 {
            (score*1_000_000.0) as i64
        } else {
            (-score*1_000_000.0) as i64
        }
    }

    pub fn calculate_move(&mut self, depth: usize) -> Move {
        let moves = self.possible_moves();
        println!("Search depth:  {}", depth);
        let alpha = i64::min_value();
        let beta = i64::max_value();
        let start = ::std::time::Instant::now();
        let mov = moves.into_iter().max_by_key(|&mov| {
            self.state.move_piece(mov);
            let v = self.search_min(alpha, beta, depth);
            self.state.move_piece(mov.inverse());
            v
        }).unwrap();

        let end = ::std::time::Instant::now();
        let elapsed = end-start;
        println!("Visited nodes: {}", self.visited_nodes);
        println!("V. leaf nodes: {}", self.visited_leaf_nodes);
        println!("alpha cutoffs: {}", self.alpha_cutoffs);
        println!("beta cutoffs:  {}", self.beta_cutoffs);
        let secs = elapsed.as_secs();
        println!("Time: {}:{}", secs / 60, (secs % 60) as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0);
        println!("");

        mov
    }
}

