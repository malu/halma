extern crate halma;
extern crate serde_json;

use std::io::{Read, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};

use halma::*;

struct EngineDefinition {
    name: String,
    path: String,
}

impl EngineDefinition {
    fn new(name: &str, path: &str) -> Self {
        EngineDefinition {
            name: name.to_owned(),
            path: path.to_owned(),
        }
    }

    fn spawn(&self) -> Engine {
        let child = Command::new(&self.path).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        Engine {
            stdin: child.stdin.unwrap(),
            stdout: child.stdout.unwrap(),
        }
    }
}

struct Engine {
    stdin: ChildStdin,
    stdout: ChildStdout,
}

impl Engine {
    fn setup(&mut self, state: &GameState) {
        self.expect_ok_response(format!("setup {}", serde_json::to_string(state).unwrap()));
    }

    fn make_move(&mut self, mov: Move) {
        self.expect_ok_response(format!("move {}", serde_json::to_string(&mov).unwrap()));
    }

    fn millis(&mut self, millis: u64) {
        self.expect_ok_response(format!("millis {}", millis));
    }

    fn quit(&mut self) {
        writeln!(self.stdin, "quit").unwrap();
    }

    fn getmove(&mut self) -> Move {
        writeln!(self.stdin, "getmove").unwrap();

        let mut response: [u8; 32] = [0; 32];
        let len = self.stdout.read(&mut response).unwrap();
        let json = std::str::from_utf8(&response[0..len]).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    fn expect_ok_response(&mut self, cmd: String) {
        writeln!(self.stdin, "{}", cmd).unwrap();

        let mut response: [u8; 3] = [0; 3];
        self.stdout.read_exact(&mut response).unwrap();
        if std::str::from_utf8(&response).unwrap() != "ok\n" {
            panic!("Did not receive 'ok'. Received '{:?}'", response);
        }
    }
}

fn run(engines: Vec<EngineDefinition>, rounds: usize) {
    let mut results: Vec<Vec<Vec<Outcome>>> = Vec::new();
    for round in 0..rounds {
        results.push(Vec::new());
        for (i, engine0) in engines.iter().enumerate() {
            results[round].push(Vec::new());
            for (j, engine1) in engines.iter().enumerate() {
                if i == j {
                    results[round][i].push(Outcome::Draw);
                    continue;
                }

                let mut ai0 = engine0.spawn();
                let mut ai1 = engine1.spawn();

                results[round][i].push(run_single(&mut ai0, &mut ai1, 1000));
                ai0.quit();
                ai1.quit();
            }
        }

        if round + 1 == rounds {
            println!("");
            println!("Tournament over.");
            println!("Final standings:");
        } else {
            println!("Round {} over.", round+1);
            println!("Current standings:");
        }

        let mut standings = engines.iter().enumerate().map(|(i, engine)| {
            let mut wins = 0;
            let mut losses = 0;
            let mut draws = 0;

            for prev_round in 0..round+1 {
                for j in 0..engines.len() {
                    if i == j {
                        continue;
                    }

                    match results[prev_round][i][j] {
                        Outcome::Win => wins += 1,
                        Outcome::Loss => losses += 1,
                        Outcome::Draw => draws += 1,
                    }

                    match results[prev_round][j][i] {
                        Outcome::Win => losses += 1,
                        Outcome::Loss => wins += 1,
                        Outcome::Draw => draws += 1,
                    }
                }
            }
            
            (&engine.name, wins, losses, draws)
        }).collect::<Vec<_>>();
        standings.sort_by_key(|&(_, wins, losses, _)| -(wins-losses));

        let engine_len = ::std::cmp::max(6, engines.iter().map(|e| e.name.len()).max().unwrap());
        println!("{:>width$} | Wins | Losses | Draws", "Engine", width = engine_len);
        for (name, wins, losses, draws) in standings {
            println!("{:>width$} | {:>4} | {:>6} | {:>5}", name, wins, losses, draws, width = engine_len);
        }
    }
}

enum Outcome {
    Win,
    Loss,
    Draw,
}

fn run_single(ai0: &mut Engine, ai1: &mut Engine, max_plies: usize) -> Outcome {
    let mut game = Game::default();
    ai0.setup(game.state());
    ai1.setup(game.state());
    ai0.millis(500);
    ai1.millis(500);

    let mut plies = 0;
    loop {
        if game.state().won(1) {
            return Outcome::Loss;
        }

        let mov = ai0.getmove();
        game.move_piece(mov);
        ai0.make_move(mov);
        ai1.make_move(mov);

        plies += 1;
        if plies > max_plies {
            return Outcome::Draw;
        }

        if game.state().won(0) {
            return Outcome::Win;
        }

        let mov = ai1.getmove();
        game.move_piece(mov);
        ai0.make_move(mov);
        ai1.make_move(mov);

        plies += 1;
        if plies > max_plies {
            return Outcome::Draw;
        }
    }
}

fn main() {
    let mut names = Vec::new();
    let mut paths = Vec::new();

    for (i, value) in ::std::env::args().skip(1).enumerate() {
        if i % 2 == 0 {
            names.push(value);
        } else {
            paths.push(value);
        }
    }

    let ais = names.iter().zip(paths).map(|(name, path)| EngineDefinition::new(name, &path)).collect();

    run(ais, 8);
}

/*
fn print_board(game: &Game) {
    for y in 0..BOARD_HEIGHT as i8 {
        if y % 2 == 0 {
            print!(" ");
        }

        for x in 0..BOARD_WIDTH as i8 {
            match game.state().get(x, y) {
                Tile::Empty => print!(". "),
                Tile::Player(i) => print!("{} ", i),
                _ => print!("  "),
            }
        }

        println!("");
    }
}
*/
