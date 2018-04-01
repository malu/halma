use {GameState, Move};

mod bitboard;
pub mod evaluation;
mod incremental_hasher;
mod internal_game_state;
mod move_list_iterator;
mod tt;

use self::evaluation::Evaluation;
use self::incremental_hasher::*;
use self::internal_game_state::*;
use self::move_list_iterator::*;
use self::tt::*;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum StopCondition {
    Depth(isize),
    Time(::std::time::Duration),
}

type Score = isize;

const WINNING_SCORE: Score = 1_000_000_000;

const ONE_PLY: isize = 1000;

pub struct AI {
    pub state: InternalGameState,
    pub print_statistics: bool,
    pub stop_condition: StopCondition,
    stop_condition_triggered: bool,
    start: ::std::time::Instant,
    main_tt: TranspositionTable,
    always_replace_tt: TranspositionTable,
    evaluation: Evaluation,
    visited_nodes: usize,
    visited_leaf_nodes: usize,
    cutoffs: usize,
    tt_lookups: usize,
    tt_hits: usize,
    pv_nullsearches: usize,
    pv_failed_nullsearches: usize,
    moves_explored: [usize; 8],

    hasher: IncrementalHasher,
    hash: IncrementalHash
}

impl AI {
    pub fn new(state: GameState) -> AI {
        AI {
            state: InternalGameState::from(state),
            print_statistics: false,
            stop_condition: StopCondition::Depth(0),
            stop_condition_triggered: false,
            start: ::std::time::Instant::now(),
            evaluation: Evaluation::from(&state),
            main_tt: TranspositionTable::new(20),
            always_replace_tt: TranspositionTable::new(10),
            visited_nodes: 0,
            visited_leaf_nodes: 0,
            cutoffs: 0,
            tt_lookups: 0,
            tt_hits: 0,
            pv_nullsearches: 0,
            pv_failed_nullsearches: 0,
            moves_explored: [0; 8],

            hasher: Default::default(),
            hash: 0,
        }
    }

    fn update_hash(&mut self, mov: InternalMove) {
        self.hash ^= self.hasher.update(self.state.current_player, mov);
    }

    pub fn make_move(&mut self, mov: Move) {
        self.internal_make_move(InternalMove::from(mov));
        self.state.ply += 1;
    }

    fn internal_make_move(&mut self, mov: InternalMove) {
        self.evaluation.make_move(self.state.current_player, mov);
        self.update_hash(mov);
        self.state.make_move(mov);
    }

    pub fn unmake_move(&mut self, mov: Move) {
        self.internal_unmake_move(InternalMove::from(mov));
        self.state.ply -= 1;
    }

    fn internal_unmake_move(&mut self, mov: InternalMove) {
        self.state.unmake_move(mov);
        self.update_hash(mov.inverse());
        self.evaluation.unmake_move(self.state.current_player, mov);
    }

    fn should_stop(&mut self, ply: isize) -> bool {
        if ply == 0 {
            return false;
        }

        if self.stop_condition_triggered {
            return true;
        }

        if self.visited_nodes & 0x7FF == 0 {
            if let StopCondition::Time(dur) = self.stop_condition {
                let time_taken = ::std::time::Instant::now() - self.start;
                let remaining = dur.checked_sub(time_taken);
                if remaining == None || remaining.unwrap() < ::std::time::Duration::new(0, ply as u32*4*1000*1000) {
                    self.stop_condition_triggered = true;
                    return true;
                }
            }
        }

        false
    }

    fn search_pv(&mut self, ply: isize, alpha: Score, beta: Score, depth: isize) -> Score {
        if self.should_stop(ply) {
            return self.evaluation.evaluate(self.state.current_player);
        }

        self.visited_nodes += 1;

        // 1. Check if we lost.
        if self.state.won(1-self.state.current_player) {
            return -WINNING_SCORE+ply;
        }

        // 2. Check if we ran out of depth and have to evaluate the position staticly.
        if depth < ONE_PLY {
            self.visited_leaf_nodes += 1;
            return self.evaluation.evaluate(self.state.current_player);
        }

        // A list of known good moves. These will be evaluated first to get early curoffs of alpha
        // increases.
        let mut known_good_moves = Vec::new();

        // 3. Lookup current position in transposition table. If we encountered this position
        //    before, previous evaluations or best moves are useful to get an early beta cutoff.
        if let Some((tt_score, tt_mov)) = self.get_transposition(alpha, beta, depth) {
            self.tt_hits += 1;

            // tt_score is not None if the position in the transposition table was evaluated to a
            // higher depth. If in that case the score is also exact, we return with this score.
            // Otherwise we add it to our list of known good moves.
            if let Some((score, exact)) = tt_score {
                if exact {
                    self.cutoffs += 1;
                    return score;
                }
            }

            known_good_moves.push(tt_mov);
        }

        // We score the moves (for ordering purposes) by how far they advance along the board.
        let current_player = self.state.current_player;
        let score_move_order = |mov: InternalMove| -> isize {
            if current_player == 0 {
                mov.to as isize - mov.from as isize
            } else {
                mov.from as isize - mov.to as isize
            }
        };

        // Tracks the number of moves tried in this position.
        let mut moves_explored = 0;

        // Whether we found any move which increases alpha and did not exceed beta. After a move
        // increased alpha, we search all remaining moves using a null-window first and only do a
        // full-window research it we failed high.
        let mut raised_alpha = false;
        let mut alpha = alpha;

        // The best response we found.
        let mut best_move = None;

        let moves = known_good_moves.into_iter().chain(self.state.possible_moves().order(8, score_move_order));
        // 4. Evaluate remaining moves. We first try the 8 highest rated moves (with respect to the
        //    move ordering score above). If we did not get a beta cutoff during these 8 moves, we
        //    try the remaining moves in any order because the move ordering seems bad and we give
        //    up sorting.
        for mov in moves {
            self.internal_make_move(mov);
            let score;

            // Only the first move is evaluated with maximum depth. All other moves are first
            // evaluated using a null window and a shallower depth. If the null window evaluation
            // fails high, we retry using the full window.
            if !raised_alpha {
                score = -self.search_pv(ply+1, -beta, -alpha, depth-ONE_PLY);
            } else {
                self.pv_nullsearches += 1;
                let null_score = -self.search_null(ply+1, -alpha, depth-ONE_PLY);
                if null_score > alpha {
                    self.pv_failed_nullsearches += 1;
                    score = -self.search_pv(ply+1, -beta, -alpha, depth-ONE_PLY);
                } else {
                    score = null_score;
                }
            }
            self.internal_unmake_move(mov);
            moves_explored += 1;

            if score >= beta {
                self.cutoffs += 1;
                self.insert_transposition(ScoreType::LowerBound(beta), Some(mov), depth, true);
                self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
                return beta;
            }

            if score > alpha {
                raised_alpha = true;
                best_move = Some(mov);
                alpha = score;
            }
        }

        if raised_alpha {
            self.insert_transposition(ScoreType::Exact(alpha), best_move, depth, true);
        } else {
            self.insert_transposition(ScoreType::UpperBound(alpha), best_move, depth, true);
        }

        self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
        alpha
    }

    fn search_null(&mut self, ply: isize, beta: Score, depth: isize) -> Score {
        if self.should_stop(ply) {
            return self.evaluation.evaluate(self.state.current_player);
        }

        self.visited_nodes += 1;

        // 1. Check if we lost.
        if self.state.won(1-self.state.current_player) {
            return -WINNING_SCORE+ply;
        }

        // 2. Check if we ran out of depth and have to evaluate the position staticly.
        if depth < ONE_PLY {
            self.visited_leaf_nodes += 1;
            return self.evaluation.evaluate(self.state.current_player);
        }

        let alpha = beta-1;

        // A list of known good moves. These will be evaluated first to get early curoffs of alpha
        // increases.
        let mut known_good_moves = Vec::new();

        // 3. Lookup current position in transposition table. If we encountered this position
        //    before, previous evaluations or best moves are useful to get an early beta cutoff.
        if let Some((tt_score, tt_mov)) = self.get_transposition(alpha, beta, depth) {
            self.tt_hits += 1;

            // tt_score is not None if the position in the transposition table was evaluated to a
            // higher depth. In that case we will not reevaluate but use the score as is. Otherwise
            // evaluate this move as any other move.
            if let Some((score, exact)) = tt_score {
                if exact {
                    self.cutoffs += 1;
                    return score;
                }

                if score >= beta {
                    self.cutoffs += 1;
                    return beta;
                }
            } else {
                known_good_moves.push(tt_mov);
            }

        }

        // We score the moves (for ordering purposes) by how far they advance along the board.
        let current_player = self.state.current_player;
        let score_move_order = |mov: InternalMove| -> isize {
            if current_player == 0 {
                mov.to as isize - mov.from as isize
            } else {
                mov.from as isize - mov.to as isize
            }
        };

        let moves = known_good_moves.into_iter().chain(self.state.possible_moves().order(8, score_move_order));
        // 4. Evaluate remaining moves. We first try the 8 highest rated moves (with respect to the
        //    move ordering score above). If we did not get a beta cutoff during these 8 moves, we
        //    try the remaining moves in any order because the move ordering seems bad and we give
        //    up sorting.
        for mov in moves {
            self.internal_make_move(mov);
            let score = -self.search_null(ply+1, -alpha, depth-ONE_PLY);
            self.internal_unmake_move(mov);

            if score >= beta {
                self.cutoffs += 1;
                self.insert_transposition(ScoreType::LowerBound(beta), Some(mov), depth, false);
                return beta;
            }
        }

        alpha
    }

    fn insert_transposition(&mut self, evaluation: ScoreType, best_move: Option<InternalMove>, depth: isize, pv: bool) {
        if best_move == None {
            return;
        }

        let transposition = Transposition {
            evaluation,
            best_move: best_move.unwrap(),
            depth,
            ply: self.state.ply,
        };

        self.always_replace_tt.insert(self.hash, self.state, transposition);

        match self.main_tt.get(self.hash, self.state) {
            None => {
                self.main_tt.insert(self.hash, self.state, transposition);
            }
            Some(old) => {
                if old.should_be_replaced_by(&transposition, pv) {
                    self.main_tt.insert(self.hash, self.state, transposition);
                }
            }
        }
    }

    fn get_transposition(&mut self, alpha: Score, beta: Score, depth: isize) -> Option<(Option<(Score, bool)>, InternalMove)> {
        self.tt_lookups += 1;
        let mut tt_entry = self.always_replace_tt.get(self.hash, self.state);

        if tt_entry == None {
            tt_entry = self.main_tt.get(self.hash, self.state);
        }

        if let Some(transposition) = tt_entry {
            let mov = transposition.best_move;

            // If the depth used to evaluate the position now is higher than the one we used
            // before, do not use the transposition table and reevaluate. Only take the best_move
            // from before as move to evaluate first.
            if transposition.depth < depth {
                return Some((None, mov));
            }

            match transposition.evaluation {
                ScoreType::Exact(score) => return Some((Some((score, true)), mov)),
                ScoreType::LowerBound(lower_bound) => {
                    if lower_bound >= beta {
                        return Some((Some((beta, false)), mov));
                    }
                }
                ScoreType::UpperBound(upper_bound) => {
                    if upper_bound <= alpha {
                        return Some((Some((alpha, false)), mov));
                    }
                }
            }
        }

        None
    }

    pub fn calculate_move(&mut self) -> Move {
        // reset statistics
        self.visited_nodes = 0;
        self.visited_leaf_nodes = 0;
        self.cutoffs = 0;
        self.tt_lookups = 0;
        self.tt_hits = 0;
        self.pv_nullsearches = 0;
        self.pv_failed_nullsearches = 0;
        self.moves_explored = [0; 8];

        self.stop_condition_triggered = false;
        //println!("Search depth:  {}", depth);
        self.start = ::std::time::Instant::now();
        let alpha = -WINNING_SCORE;
        let beta = WINNING_SCORE;
        let mut score = 0;
        for d in 1.. {
            match self.stop_condition {
                StopCondition::Depth(stop_depth) => {
                    if stop_depth < d {
                        self.stop_condition_triggered = true;
                        break;
                    }
                }
                StopCondition::Time(dur) => {
                    let time_taken = ::std::time::Instant::now() - self.start;
                    let remaining = dur.checked_sub(time_taken);
                    if remaining == None || remaining.unwrap() < ::std::time::Duration::new(0, 50_000_000) {
                        self.stop_condition_triggered = true;
                        if self.print_statistics {
                            println!("Stopping search after depth {}", d-1);
                        }
                        break;
                }
            }
            }

            score = self.search_pv(0, alpha, beta, d*ONE_PLY);
        }

        let mov;
        if let Some((_, pvmove)) = self.get_transposition(alpha, beta, 1) {
            mov = pvmove;
        } else {
            panic!("No PV entry in transposition table");
        }

        let end = ::std::time::Instant::now();
        let elapsed = end-self.start;
        let secs = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0;
        let interior_nodes = self.visited_nodes - self.visited_leaf_nodes;
        if self.print_statistics {
            println!("  nodes | total   {} ({:.3} knodes/s)", self.visited_nodes, self.visited_nodes as f64 / secs / 1000.0);
            println!("        | leaf    {} ({:.2}%)", self.visited_leaf_nodes, 100.0 * self.visited_leaf_nodes as f64 / self.visited_nodes as f64);
            println!("        | inner   {} ({:.2}%)", interior_nodes, 100.0 * interior_nodes as f64 / self.visited_nodes as f64);
            println!("cutoffs | total   {} ({:.2}%)", self.cutoffs, 100.0 * self.cutoffs as f64 / interior_nodes as f64);
            println!("     TT | lookups {}", self.tt_lookups);
            println!("        | hits    {} ({:.2}%)", self.tt_hits, 100.0 * self.tt_hits as f64 / self.tt_lookups as f64);
            println!("     PV | 0-wind. {}", self.pv_nullsearches);
            println!("        | failed  {} ({:.2}%)", self.pv_failed_nullsearches, 100.0 * self.pv_failed_nullsearches  as f64 / self.pv_nullsearches as f64);
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
        }

        mov.to_move()
    }
}

