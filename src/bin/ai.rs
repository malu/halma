extern crate halma;
extern crate serde_json;

use std::io::{self, BufRead};

use halma::*;
use halma::ai::{AI, StopCondition};

fn main() {
    let mut ai = AI::new(GameState::default());

    let stdin = io::stdin();
    let lock = stdin.lock();
    for line in lock.lines() {
        let line = line.unwrap();
        if line.starts_with("quit") {
            break;
        } else if line.starts_with("setup ") {
            ai = AI::new(serde_json::from_str(&line.trim_left_matches("setup ")).unwrap());
            println!("ok");
        } else if line.starts_with("move ") {
            let mov = serde_json::from_str(&line.trim_left_matches("move ")).unwrap();
            ai.make_move(mov);
            println!("ok");
        } else if line.starts_with("getmove") {
            let mov = ai.calculate_move();
            println!("{}", serde_json::to_string(&mov).unwrap());
        } else if line.starts_with("millis ") {
            let millis: u64 = line.trim_left_matches("millis ").parse().unwrap();
            ai.stop_condition = StopCondition::Time(::std::time::Duration::new(millis / 1000, (millis % 1000) as u32*1_000_000));
            println!("ok");
        }
    }
}
