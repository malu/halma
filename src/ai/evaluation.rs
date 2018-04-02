//! The Evaluation
//!
//! The current game state is evaluated using five aspects:
//!   * The distance of the least advanced piece of each player to its destination side of the
//!     board.
//!   * The total distance of the remaining pieces to the destination side of the board.
//!   * How well the kinds of pieces match up with the kinds of the destination positions.
//!   * How centralized the pieces of each player are.
//!   * The mobility of the individual pieces.

use ::{BOARD_HEIGHT, BOARD_WIDTH, GameState, Tile};
use ai::Score;
use ai::internal_game_state::{InternalGameState, InternalMove};
use ai::bitboard::index_to_pos;

/// Caches properties of and evaluates the current game state.
///
/// Notice that because it does only recompute what changed (except at construction time via
/// `from`) it is necessary to call `make_move` and `unmake_move` after the game state has changed.
pub struct Evaluation {
    target_kinds: [[i8; 4]; 2],
    kinds: [[i8; 4]; 2],
    ys: [[i8; BOARD_HEIGHT as usize]; 2],
    dist: [isize; 2],
    dist_to_center: [[i8; BOARD_WIDTH as usize]; 2],
}

impl<'a> From<&'a GameState> for Evaluation {
    fn from(state: &GameState) -> Self {
        let mut target_kinds = [[0; 4]; 2];
        for &(x, y) in GameState::targets(0) {
            target_kinds[0][kind(x, y)] += 1;
        }

        for &(x, y) in GameState::targets(1) {
            target_kinds[1][kind(x, y)] += 1;
        }

        let mut kinds = [[0; 4]; 2];
        let mut ys = [[0; BOARD_HEIGHT as usize]; 2];
        let mut dist = [0; 2];
        let mut dist_to_center = [[0; BOARD_WIDTH as usize]; 2];
        for x in 0..BOARD_WIDTH as i8 {
            for y in 0..BOARD_HEIGHT as i8 {
                match state.get(x, y) {
                    Tile::Player(0) => {
                        kinds[0][kind(x, y)] += 1;
                        ys[0][y as usize] += 1;
                        dist[0] += BOARD_HEIGHT as isize - 1 - y as isize;
                        dist_to_center[0][distance_to_center(x, y) as usize] += 1;
                    }
                    Tile::Player(1) => {
                        kinds[1][kind(x, y)] += 1;
                        ys[1][y as usize] += 1;
                        dist[1] += y as isize;
                        dist_to_center[1][distance_to_center(x, y) as usize] += 1;
                    }
                    _ => {}
                }
            }
        }

        Evaluation {
            target_kinds,
            kinds,
            ys,
            dist_to_center,
            dist,
        }
    }
}

impl Evaluation {
    /// Updates the evaluation cache for the move `mov` of player `player`.
    pub fn make_move(&mut self, player: u8, mov: InternalMove) {
        let (fx, fy) = index_to_pos(mov.from);
        let (tx, ty) = index_to_pos(mov.to);
        self.kinds[player as usize][kind(fx, fy)] -= 1;
        self.kinds[player as usize][kind(tx, ty)] += 1;
        self.ys[player as usize][fy as usize] -= 1;
        self.ys[player as usize][ty as usize] += 1;
        if player == 0 {
            self.dist[player as usize] += (fy - ty) as isize;
        } else {
            self.dist[player as usize] += (ty - fy) as isize;
        }
        self.dist_to_center[player as usize][distance_to_center(fx, fy) as usize] -= 1;
        self.dist_to_center[player as usize][distance_to_center(tx, ty) as usize] += 1;
    }

    /// Updates the evaluation cache for the reverse move `mov` of player `player`.
    pub fn unmake_move(&mut self, player: u8, mov: InternalMove) {
        self.make_move(player, mov.inverse());
    }

    /// Calculates an evaluation score using the cached data and computes some not easily cacheable
    /// score.
    pub fn evaluate(&mut self, state: InternalGameState) -> Score {
        let mut score = 0;
        score += 100_000 * self.score_dist_last_piece() / 17;
        score += 100_000 * self.score_total_distance() / 209;
        score += 100_000 * self.score_centralization() / 100;
        score += 100_000 * self.score_kinds() / 120;
        score += self.score_mobility(state) * 2;

        if state.current_player == 0 {
            score
        } else {
            -score
        }
    }

    fn score_kinds(&self) -> Score {
        let p0 = self.kinds[0].iter().zip(&self.target_kinds[0]).map(|(&have, &target): (&i8, &i8)| (target-have).abs()).sum::<i8>() as Score;
        let p1 = self.kinds[1].iter().zip(&self.target_kinds[1]).map(|(&have, &target): (&i8, &i8)| (target-have).abs()).sum::<i8>() as Score;
        p1 - p0
    }

    fn score_total_distance(&self) -> Score {
        let p0 = self.dist[0];
        let p1 = self.dist[1];
        p1 - p0 - self.score_dist_last_piece()
    }

    fn score_dist_last_piece(&self) -> Score {
        let p0 = self.ys[0].iter().rev().enumerate().rev().find(|&(_dist, &count)| count > 0).unwrap().0 as isize;
        let p1 = self.ys[1].iter().enumerate().rev().find(|&(_dist, &count)| count > 0).unwrap().0 as isize;
        p1 - p0
    }

    fn score_centralization(&self) -> Score {
        let p0 = self.dist_to_center[0].iter().enumerate().map(|(dist, &count)| ::std::cmp::max(0, dist as Score-1) as Score*count as Score).sum::<Score>();
        let p1 = self.dist_to_center[1].iter().enumerate().map(|(dist, &count)| ::std::cmp::max(0, dist as Score-1) as Score*count as Score).sum::<Score>();
        p1 - p0
    }

    fn score_mobility(&self, state: InternalGameState) -> Score {
        let p0: Score = state.pieces[0].ones().map(|i| state.reachable_from(i).popcount() as Score).sum();
        let p1: Score = state.pieces[1].ones().map(|i| state.reachable_from(i).popcount() as Score).sum();
        p0 - p1
    }
}

fn distance_to_center(x: i8, y: i8) -> i8 {
    ::std::cmp::min((6-x).abs(), (x-(6+y%2)).abs())
}

fn kind(x: i8, y: i8) -> usize {
    (2*((x + y/2)%2) + y%2) as usize
}

mod tests {
    #[test]
    fn test_kinds() {
        use ai::evaluation_cache::kind;

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

