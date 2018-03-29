use ::{BOARD_HEIGHT, BOARD_WIDTH, GameState, Move, Tile};
use ai::Score;

pub struct EvaluationCache {
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
        for &(x, y) in GameState::targets(0) {
            p0_target_kinds[kind(x, y)] += 1;
        }

        let mut p1_target_kinds = [0; 4];
        for &(x, y) in GameState::targets(1) {
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

    pub fn update(&mut self, player: u8, mov: Move) {
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
    pub fn score_kinds(&self) -> Score {
        let p0 = self.p0_kinds.iter().zip(&self.p0_target_kinds).map(|(&have, &target): (&i8, &i8)| (target-have).abs()).sum::<i8>() as Score;
        let p1 = self.p1_kinds.iter().zip(&self.p1_target_kinds).map(|(&have, &target): (&i8, &i8)| (target-have).abs()).sum::<i8>() as Score;
        // p1-p0 lies in [-12, 12]
        100_000 * (p1 - p0) / 12
    }

    pub fn score_total_distance(&self) -> Score {
        let p0 = self.p0_dist;
        let p1 = self.p1_dist;
        // p1 - p0 lies between [-209, 209]
        100_000*(p1 - p0) / 209
    }

    pub fn score_dist_first_piece(&self) -> Score {
        let p0 = self.p0_ys.iter().rev().enumerate().find(|&(_dist, &count)| count > 0).unwrap().0 as isize;
        let p1 = self.p1_ys.iter().enumerate().find(|&(_dist, &count)| count > 0).unwrap().0 as isize;

        100_000*(p1 - p0) / 13
    }

    pub fn score_dist_last_piece(&self) -> Score {
        let p0 = self.p0_ys.iter().rev().enumerate().rev().find(|&(_dist, &count)| count > 0).unwrap().0 as isize;
        let p1 = self.p1_ys.iter().enumerate().rev().find(|&(_dist, &count)| count > 0).unwrap().0 as isize;

        100_000*(p1 - p0) / 17
    }

    pub fn score_centralization(&self) -> Score {
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

