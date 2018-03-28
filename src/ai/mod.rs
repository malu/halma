use {BOARD_HEIGHT, BOARD_WIDTH, GameState, Move, Tile};

type IncrementalHash = usize;
type Score = isize;

const ONE_PLY: isize = 1000;

struct IncrementalHasher {
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

pub struct AI {
    pub state: GameState,
    pub print_statistics: bool,
    transpositions: TranspositionTable,
    evaluation_cache: EvaluationCache,
    visited_nodes: usize,
    visited_leaf_nodes: usize,
    beta_cutoffs: usize,
    tt_lookups: usize,
    tt_hits: usize,
    tt_cutoffs: usize,
    pv_nullsearches: usize,
    pv_failed_nullsearches: usize,
    moves_explored: [usize; 8],

    hasher: IncrementalHasher,
    hash: IncrementalHash
}

impl AI {
    pub fn new(state: GameState) -> AI {
        AI {
            state,
            print_statistics: false,
            evaluation_cache: EvaluationCache::from(&state),
            transpositions: TranspositionTable::default(),
            visited_nodes: 0,
            visited_leaf_nodes: 0,
            beta_cutoffs: 0,
            tt_lookups: 0,
            tt_hits: 0,
            tt_cutoffs: 0,
            pv_nullsearches: 0,
            pv_failed_nullsearches: 0,
            moves_explored: [0; 8],

            hasher: Default::default(),
            hash: 0,
        }
    }

    fn update_hash(&mut self, mov: Move) {
        let from;
        let to;
        if self.state.current_player == 0 {
            from = self.hasher.tile_hashes[mov.from.0 as usize][mov.from.1 as usize].0;
            to = self.hasher.tile_hashes[mov.to.0 as usize][mov.to.1 as usize].0;
        } else {
            from = self.hasher.tile_hashes[mov.from.0 as usize][mov.from.1 as usize].1;
            to = self.hasher.tile_hashes[mov.to.0 as usize][mov.to.1 as usize].1;
        }
        self.hash ^= from ^ to ^ self.hasher.to_move_hash;
    }

    pub fn make_move(&mut self, mov: Move) {
        self.internal_make_move(mov);
        self.state.ply += 1;
    }

    fn internal_make_move(&mut self, mov: Move) {
        self.evaluation_cache.update(self.state.current_player, mov);
        self.update_hash(mov);
        self.state.move_piece(mov);
    }

    pub fn unmake_move(&mut self, mov: Move) {
        self.internal_unmake_move(mov);
        self.state.ply -= 1;
    }

    fn internal_unmake_move(&mut self, mov: Move) {
        self.state.move_piece(mov.inverse());
        self.evaluation_cache.update(self.state.current_player, mov.inverse());
        self.update_hash(mov.inverse());
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
        result
    }

    fn search_negamax(&mut self, ply: isize, alpha: Score, beta: Score, depth: isize) -> Score {
        self.visited_nodes += 1;

        // 1. Check if we lost.
        if self.state.won(1-self.state.current_player) {
            return -Score::max_value()+ply;
        }

        // 2. Check if we ran out of depth and have to evaluate the position staticly.
        if depth <= 0 {
            return self.evaluate_position();
        }

        // Tracks the number of moves tried in this position. We use this to distinguish between
        // the first move we try (which we expect to be the best/principal variation) and the other
        // moves (which we first try to evaluate using a null window). Additionally, the statistic
        // how often we explore 0, 1, .., 6 and 7 or more moves is nice.
        let mut moves_explored = 0;
        let mut alpha = alpha;

        // Whether we found any move which increases alpha and did not exceed beta.
        let mut is_exact = false;

        // The best response we found.
        let mut best_move = None;

        // 3. Lookup current position in transposition table. If we encountered this position
        //    before, previous evaluations or best moves are useful to get an early beta cutoff
        //    (which in this case will be tracked as a transposition table cutoff in the statistic).
        if let Some((tt_score, tt_mov)) = self.get_transposition(alpha, beta, depth) {
            self.tt_hits += 1;

            // tt_score is not None if the position in the transposition table was evaluated to a
            // higher depth. In that case we will not reevaluate but use the score as is. Otherwise
            // evaluate this move as any other move.
            let tt_move_score;
            if let Some(score) = tt_score {
                tt_move_score = score;
            } else {
                // There is no possible path in which we evaluate another move first. Hence we can
                // skip the check whether this is the first evaluated move.
                assert!(moves_explored == 0);
                self.internal_make_move(tt_mov);
                tt_move_score = -self.search_negamax(ply+1, -beta, -alpha, depth-ONE_PLY);
                self.internal_unmake_move(tt_mov);
                moves_explored += 1;
            }

            // In this case we track beta cutoffs as transposition table cutoffs.
            if tt_move_score >= beta {
                self.tt_cutoffs += 1;
                self.insert_transposition(Evaluation::LowerBound(beta), Some(tt_mov), depth);
                self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
                return beta;
            }

            if tt_move_score > alpha {
                is_exact = true;
                best_move = Some(tt_mov);
                alpha = tt_move_score;
            }
        }

        // We score the moves (for ordering purposes) by how far they advance along the board.
        let current_player = self.state.current_player;
        let score_move_order = |mov: Move| -> isize {
            if current_player == 0 {
                mov.to.1 as isize - mov.from.1 as isize
            } else {
                mov.from.1 as isize - mov.to.1 as isize
            }
        };

        // 4. Evaluate remaining moves. We first try the 8 highest rated moves (with respect to the
        //    move ordering score above). If we did not get a beta cutoff during these 8 moves, we
        //    try the remaining moves in any order because the move ordering seems bad and we give
        //    up sorting.
        for mov in self.possible_moves().order(8, score_move_order) {
            self.internal_make_move(mov);
            let score;

            // Only the first move is evaluated with maximum depth. All other moves are first
            // evaluated using a null window and a shallower depth. If the null window evaluation
            // fails high, we retry using the full window.
            if moves_explored == 0 {
                score = -self.search_negamax(ply+1, -beta, -alpha, depth-ONE_PLY);
            } else {
                self.pv_nullsearches += 1;
                let null_score = -self.search_negamax(ply+1, -alpha-1, -alpha, depth-ONE_PLY*5/3);
                if null_score > alpha {
                    self.pv_failed_nullsearches += 1;
                    score = -self.search_negamax(ply+1, -beta, -alpha, depth-ONE_PLY);
                } else {
                    score = null_score;
                }
            }
            self.internal_unmake_move(mov);
            moves_explored += 1;

            if score >= beta {
                self.beta_cutoffs += 1;
                self.insert_transposition(Evaluation::LowerBound(beta), Some(mov), depth);
                self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
                return beta;
            }

            if score > alpha {
                is_exact = true;
                best_move = Some(mov);
                alpha = score;
            }
        }

        if is_exact {
            self.insert_transposition(Evaluation::Exact(alpha), best_move, depth);
        } else {
            self.insert_transposition(Evaluation::UpperBound(alpha), best_move, depth);
        }

        self.moves_explored[::std::cmp::min(7, moves_explored)] += 1;
        alpha
    }

    fn evaluate_position(&mut self) -> Score {
        self.visited_leaf_nodes += 1;

        let mut score = 0;
        let score_dist_last_piece = self.evaluation_cache.score_dist_last_piece();
        score += score_dist_last_piece;

        let score_total_distance = self.evaluation_cache.score_total_distance();
        score += score_total_distance;

        let score_center = self.evaluation_cache.score_centralization();
        score += score_center/3;

        let score_kinds = self.evaluation_cache.score_kinds();
        score += score_kinds/6;

        if self.state.current_player == 0 {
            score
        } else {
            -score
        }
    }

    fn insert_transposition(&mut self, evaluation: Evaluation, best_move: Option<Move>, depth: isize) {
        if best_move == None {
            return;
        }

        let transposition = Transposition {
            evaluation,
            best_move: best_move.unwrap(),
            depth,
            ply: self.state.ply,
        };

        let old = self.transpositions.get(self.hash, self.state);
        if old.is_none() {
            self.transpositions.insert(self.hash, self.state, transposition);
            return;
        }

        if old.unwrap().ply + 6 < self.state.ply {
            self.transpositions.insert(self.hash, self.state, transposition);
        }

        if old.unwrap().depth <= depth {
            self.transpositions.insert(self.hash, self.state, transposition);
        }

    }

    fn get_transposition(&mut self, alpha: Score, beta: Score, depth: isize) -> Option<(Option<Score>, Move)> {
        self.tt_lookups += 1;
        if let Some(transposition) = self.transpositions.get(self.hash, self.state) {
            let mov = transposition.best_move;

            // If the depth used to evaluate the position now is higher than the one we used
            // before, do not use the transposition table and reevaluate. Only take the best_move
            // from before as move to evaluate first.
            if transposition.depth < depth {
                return Some((None, mov));
            }

            match transposition.evaluation {
                Evaluation::Exact(score) => return Some((Some(score), mov)),
                Evaluation::LowerBound(lower_bound) => {
                    if lower_bound >= beta {
                        return Some((Some(beta), mov));
                    }
                }
                Evaluation::UpperBound(upper_bound) => {
                    if upper_bound <= alpha {
                        return Some((Some(alpha), mov));
                    }
                }
            }
        }

        None
    }

    pub fn calculate_move(&mut self, depth: isize) -> Move {
        // reset statistics
        self.visited_nodes = 0;
        self.visited_leaf_nodes = 0;
        self.beta_cutoffs = 0;
        self.tt_lookups = 0;
        self.tt_hits = 0;
        self.tt_cutoffs = 0;
        self.pv_nullsearches = 0;
        self.pv_failed_nullsearches = 0;
        self.transpositions.insertion = 0;
        self.transpositions.replace = 0;
        self.transpositions.update = 0;
        self.moves_explored = [0; 8];

        //println!("Search depth:  {}", depth);
        let start = ::std::time::Instant::now();
        let mut moves = self.possible_moves();
        let num_moves = moves.len();
        let mut score = Score::min_value();

        let current_player = self.state.current_player;
        let score_move_order = |mov: Move| -> isize {
            if current_player == 0 {
                mov.to.1 as isize - mov.from.1 as isize
            } else {
                mov.from.1 as isize - mov.to.1 as isize
            }
        };

        let mut max_move = moves[0];

        for d in 0..depth {
            let mut moves_explored = 0;
            let mut alpha = -Score::max_value();
            let mut beta = Score::max_value();

            let mut max_score = -Score::max_value();
            score = Score::min_value();

            for i in 0..num_moves {
                if moves_explored < 8 {
                    let mut max_i = i;
                    let mut max_move_score = score_move_order(moves[max_i]);

                    for j in i+1..num_moves {
                        let move_score = score_move_order(moves[j]);
                        if move_score > max_move_score {
                            max_i = j;
                            max_move_score = move_score;
                        }
                    }

                    moves.swap(i, max_i);
                }

                let mov = moves[i];
                self.internal_make_move(mov);
                let v;
                if moves_explored == 0 {
                    v = -self.search_negamax(1, -beta, -alpha, d*ONE_PLY);
                } else {
                    self.pv_nullsearches += 1;
                    let null_v = -self.search_negamax(1, -alpha-1, -alpha, d*ONE_PLY);
                    if null_v > alpha {
                        self.pv_failed_nullsearches += 1;
                        v = -self.search_negamax(1, -beta, -alpha, d*ONE_PLY);
                    } else {
                        v = null_v;
                    }
                }
                self.internal_unmake_move(mov);
                moves_explored += 1;

                if v > alpha {
                    max_move = mov;
                    max_score = v;
                    alpha = v;
                }

                score = ::std::cmp::max(alpha, score);

            }
        }

        let end = ::std::time::Instant::now();
        let elapsed = end-start;
        let secs = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0;
        let interior_nodes = self.visited_nodes - self.visited_leaf_nodes;
        let total_cutoffs = self.tt_cutoffs + self.beta_cutoffs;
        if self.print_statistics {
            println!("  nodes | total   {} ({:.3} knodes/s)", self.visited_nodes, self.visited_nodes as f64 / secs / 1000.0);
            println!("        | leaf    {} ({:.2}%)", self.visited_leaf_nodes, 100.0 * self.visited_leaf_nodes as f64 / self.visited_nodes as f64);
            println!("        | inner   {} ({:.2}%)", interior_nodes, 100.0 * interior_nodes as f64 / self.visited_nodes as f64);
            println!("cutoffs | beta    {} ({:.2}%)", self.beta_cutoffs, 100.0 * self.beta_cutoffs as f64 / interior_nodes as f64);
            println!("        | TT      {} ({:.2}%)", self.tt_cutoffs, 100.0 * self.tt_cutoffs as f64 / interior_nodes as f64);
            println!("        | total   {} ({:.2}%)", total_cutoffs, 100.0 * total_cutoffs as f64 / interior_nodes as f64);
            println!("     TT | lookups {}", self.tt_lookups);
            println!("        | hits    {} ({:.2}%)", self.tt_hits, 100.0 * self.tt_hits as f64 / self.tt_lookups as f64);
            println!("        | cutoffs {} ({:.2}%)", self.tt_cutoffs, 100.0 * self.tt_cutoffs as f64 / self.tt_hits as f64);
            println!("        | size    {} ({} MB)", self.transpositions.len(), (self.transpositions.len() * ::std::mem::size_of::<Option<(GameState, Transposition)>>()) / (1024*1024));
            println!("        | insert  {}", self.transpositions.insertion);
            println!("        | update  {} ({:.2}%)", self.transpositions.update, 100.0 * self.transpositions.update as f64 / self.transpositions.insertion as f64);
            println!("        | replace {} ({:.2}%)", self.transpositions.replace, 100.0 * self.transpositions.replace as f64 / self.transpositions.insertion as f64);
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

        max_move
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Evaluation {
    Exact(Score),
    LowerBound(Score),
    UpperBound(Score)
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct Transposition {
    evaluation: Evaluation,
    best_move: Move,
    depth: isize,
    ply: usize,
}

const TT_BITS: usize = 16;
const TT_SIZE: usize = 1 << TT_BITS;

struct TranspositionTable {
    table: Vec<Option<(GameState, Transposition)>>,
    insertion: usize,
    replace: usize,
    update: usize,
}

impl Default for TranspositionTable {
    fn default() -> Self {
        let mut table = Vec::with_capacity(TT_SIZE);

        for _ in 0..TT_SIZE {
            table.push(None);
        }

        TranspositionTable {
            table,
            insertion: 0,
            replace: 0,
            update: 0,
        }
    }
}

impl TranspositionTable {
    fn len(&self) -> usize {
        self.table.iter().filter(|option| option.is_some()).count()
    }

    fn get(&self, hash: IncrementalHash, state: GameState) -> Option<Transposition> {
        if let Some((tstate, t)) = self.table[hash % TT_SIZE] {
            if tstate != state {
                return None;
            } else {
                return Some(t);
            }
        }
        
        None
    }

    fn insert(&mut self, hash: IncrementalHash, state: GameState, transposition: Transposition) {
        self.insertion += 1;
        if let Some((tstate, _)) = self.table[hash % TT_SIZE] {
            if tstate == state {
                self.update += 1;
            } else {
                self.replace += 1;
            }
        }

        self.table[hash % TT_SIZE] = Some((state, transposition));
    }
}

trait MoveList {
    fn order<F>(self, n: usize, score_fn: F) -> OrderedMovesIterator<F> where F: Fn(Move) -> Score;
}

impl MoveList for Vec<Move> {
    fn order<F>(self, n: usize, score_fn: F) -> OrderedMovesIterator<F> where F: Fn(Move) -> Score {
        OrderedMovesIterator {
            moves: self,
            index: 0,
            n,
            score_fn,
        }
    }
}

struct OrderedMovesIterator<F> {
    moves: Vec<Move>,
    n: usize,
    index: usize,
    score_fn: F,
}

impl<F> Iterator for OrderedMovesIterator<F> where F: Fn(Move) -> Score {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.moves.len() {
            return None;
        }

        if self.index < self.n {
            let num_moves = self.moves.len();
            let mut max_i = self.index;
            let mut max_move_score = (self.score_fn)(self.moves[max_i]);

            for j in self.index+1..num_moves {
                let move_score = (self.score_fn)(self.moves[j]);
                if move_score > max_move_score {
                    max_i = j;
                    max_move_score = move_score;
                }
            }

            self.moves.swap(self.index, max_i);
        }

        let mov = self.moves[self.index];
        self.index += 1;
        Some(mov)
    }
}

struct EvaluationCache {
    p0_target_kinds: [i8; 4],
    p1_target_kinds: [i8; 4],
    p0_kinds: [i8; 4],
    p1_kinds: [i8; 4],
    p0_ys: [i8; BOARD_HEIGHT as usize],
    p1_ys: [i8; BOARD_HEIGHT as usize],
    p0_dist: isize,
    p1_dist: isize,
    p0_dist_to_center: [i8; BOARD_WIDTH as usize],
    p1_dist_to_center: [i8; BOARD_WIDTH as usize],
}

impl<'a> From<&'a GameState> for EvaluationCache {
    fn from(state: &GameState) -> Self {
        let mut p0_target_kinds = [0; 4];
        for &(x, y) in state.targets(0) {
            p0_target_kinds[kind(x, y)] += 1;
        }

        let mut p1_target_kinds = [0; 4];
        for &(x, y) in state.targets(1) {
            p1_target_kinds[kind(x, y)] += 1;
        }

        let mut p0_kinds = [0; 4];
        let mut p1_kinds = [0; 4];
        let mut p0_ys = [0; BOARD_HEIGHT as usize];
        let mut p1_ys = [0; BOARD_HEIGHT as usize];
        let mut p0_dist = 0;
        let mut p1_dist = 0;
        let mut p0_dist_to_center = [0; BOARD_WIDTH as usize];
        let mut p1_dist_to_center = [0; BOARD_WIDTH as usize];
        for x in 0..BOARD_WIDTH as i8 {
            for y in 0..BOARD_HEIGHT as i8 {
                let tile = state.get(x, y);
                if tile == Tile::Player(0) {
                    p0_kinds[kind(x, y)] += 1;
                    p0_ys[y as usize] += 1;
                    p0_dist += BOARD_HEIGHT as isize - 1 - y as isize;
                    p0_dist_to_center[EvaluationCache::dist_to_center(x, y) as usize] += 1;
                } else if tile == Tile::Player(1) {
                    p1_kinds[kind(x, y)] += 1;
                    p1_ys[y as usize] += 1;
                    p1_dist += y as isize;
                    p1_dist_to_center[EvaluationCache::dist_to_center(x, y) as usize] += 1;
                }
            }
        }

        EvaluationCache {
            p0_target_kinds,
            p1_target_kinds,
            p0_kinds,
            p1_kinds,
            p0_ys,
            p1_ys,
            p0_dist_to_center,
            p1_dist_to_center,
            p0_dist,
            p1_dist,
        }
    }
}

impl EvaluationCache {
    fn dist_to_center(x: i8, y: i8) -> i8 {
        ::std::cmp::min((6-x).abs(), (x-(6+y%2)).abs())
    }

    fn update(&mut self, player: u8, mov: Move) {
        if player == 0 {
            self.p0_kinds[kind(mov.from.0, mov.from.1)] -= 1;
            self.p0_kinds[kind(mov.to.0, mov.to.1)] += 1;
            self.p0_ys[mov.from.1 as usize] -= 1;
            self.p0_ys[mov.to.1 as usize] += 1;
            self.p0_dist += (mov.from.1 - mov.to.1) as isize;
            self.p0_dist_to_center[EvaluationCache::dist_to_center(mov.from.0, mov.from.1) as usize] -= 1;
            self.p0_dist_to_center[EvaluationCache::dist_to_center(mov.to.0, mov.to.1) as usize] += 1;
        } else if player == 1 {
            self.p1_kinds[kind(mov.from.0, mov.from.1)] -= 1;
            self.p1_kinds[kind(mov.to.0, mov.to.1)] += 1;
            self.p1_ys[mov.from.1 as usize] -= 1;
            self.p1_ys[mov.to.1 as usize] += 1;
            self.p1_dist += (mov.to.1 - mov.from.1) as isize;
            self.p1_dist_to_center[EvaluationCache::dist_to_center(mov.from.0, mov.from.1) as usize] -= 1;
            self.p1_dist_to_center[EvaluationCache::dist_to_center(mov.to.0, mov.to.1) as usize] += 1;
        }
    }

    // Score should lie between -100,000 and 100,000
    fn score_kinds(&self) -> Score {
        let p0 = self.p0_kinds.iter().zip(&self.p0_target_kinds).map(|(&have, &target): (&i8, &i8)| (target-have).abs()).sum::<i8>() as Score;
        let p1 = self.p1_kinds.iter().zip(&self.p1_target_kinds).map(|(&have, &target): (&i8, &i8)| (target-have).abs()).sum::<i8>() as Score;
        // p1-p0 lies in [-12, 12]
        100_000 * (p1 - p0) / 12
    }

    fn score_total_distance(&self) -> Score {
        let p0 = self.p0_dist;
        let p1 = self.p1_dist;
        // p1 - p0 lies between [-209, 209]
        100_000*(p1 - p0) / 209
    }

    fn score_dist_first_piece(&self) -> Score {
        let p0 = self.p0_ys.iter().rev().enumerate().find(|&(_dist, &count)| count > 0).unwrap().0 as isize;
        let p1 = self.p1_ys.iter().enumerate().find(|&(_dist, &count)| count > 0).unwrap().0 as isize;

        100_000*(p1 - p0) / 13
    }

    fn score_dist_last_piece(&self) -> Score {
        let p0 = self.p0_ys.iter().rev().enumerate().rev().find(|&(_dist, &count)| count > 0).unwrap().0 as isize;
        let p1 = self.p1_ys.iter().enumerate().rev().find(|&(_dist, &count)| count > 0).unwrap().0 as isize;

        100_000*(p1 - p0) / 17
    }

    fn score_centralization(&self) -> Score {
        let p0 = self.p0_dist_to_center.iter().enumerate().map(|(dist, &count)| ::std::cmp::max(0, dist as Score-3) as Score*count as Score).sum::<Score>();
        let p1 = self.p1_dist_to_center.iter().enumerate().map(|(dist, &count)| ::std::cmp::max(0, dist as Score-3) as Score*count as Score).sum::<Score>();
        100_000*(p1 - p0) / 100
    }
}

fn kind(x: i8, y: i8) -> usize {
    (2*((x + y/2)%2) + y%2) as usize
}

mod tests {
    #[test]
    fn test_kinds() {
        use ai::kind;

        let x = 6;
        let y = 9;

        assert_eq!(kind(x, y), kind(x+2, y));
        assert_eq!(kind(x, y), kind(x-2, y));
        assert_eq!(kind(x, y), kind(x+1, y+2));
        assert_eq!(kind(x, y), kind(x-1, y+2));
        assert_eq!(kind(x, y), kind(x+1, y-2));
        assert_eq!(kind(x, y), kind(x-1, y-2));

        assert_ne!(kind(x, y), kind(x+1, y));
        assert_ne!(kind(x, y), kind(x-1, y));
        assert_ne!(kind(x, y), kind(x+1, y+1));
        assert_ne!(kind(x, y), kind(x, y+1));
    }
}

