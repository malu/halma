extern crate rayon;
extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::gfx::primitives::DrawRenderer;

mod ai;

use ai::AI;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Tile {
    Empty,
    Invalid,
    Player(u8),
}

impl Tile {
    fn draw(self, canvas: &mut sdl2::render::WindowCanvas, board_x: i8, board_y: i8) {
        let (draw_x, draw_y) = board_space_to_screen_space(board_x, board_y);

        match self {
            Tile::Empty => {
                canvas.set_draw_color(Color::RGB(64, 64, 64));
                canvas.fill_rect(Some(sdl2::rect::Rect::new(draw_x-3, draw_y-3, 6, 6))).unwrap();
            }
            Tile::Player(id) => {
                canvas.set_draw_color(player_color(id));
                canvas.fill_rect(Some(sdl2::rect::Rect::new(draw_x-4, draw_y-4, 8, 8))).unwrap();
            }
            _ => {}
        }
    }
}

fn player_color(id: u8) -> Color {
    match id {
        1 => Color::RGB(255, 0, 0),
        2 => Color::RGB(0, 0, 255),
        _ => unimplemented!()
    }
}

pub const BOARD_WIDTH: u8 = 13;
pub const BOARD_HEIGHT: u8 = 17;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GameState {
    board: [[Tile; BOARD_HEIGHT as usize]; BOARD_WIDTH as usize],
    current_player: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Move {
    from: (i8, i8),
    to: (i8, i8),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Game {
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
        if self.is_valid_location(x, y) {
            self.board[x as usize][y as usize] = tile;
        }
    }

    fn get(&self, x: i8, y: i8) -> Tile {
        if x < 0 || y < 0 || x as u8 >= BOARD_WIDTH || y as u8 >= BOARD_HEIGHT {
            return Tile::Invalid;
        }

        self.board[x as usize][y as usize]
    }

    fn reachable_from(&self, x: i8, y: i8) -> Vec<(i8, i8)> {
        let mut result = Vec::new();
        let mut jumping_targets = vec![(x, y)];

        for &(dx, dy) in &[(-1, 0), (1, 0), (-y%2+1, 1), (-y%2, 1), (-y%2+1, -1), (-y%2, -1)] {
            if self.get(x+dx, y+dy) == Tile::Empty {
                result.push((x+dx, y+dy));
            }
        }

        while !jumping_targets.is_empty() {
            let (x, y) = jumping_targets.pop().unwrap();

            for &(dx, dy, jx, jy) in &[(-1, 0, -2, 0), (1, 0, 2, 0), (-y%2+1, 1, 1, 2), (-y%2, 1, -1, 2), (-y%2+1, -1, 1, -2), (-y%2, -1, -1, -2)] {
                if let Tile::Player(_) = self.get(x+dx, y+dy) {
                    if self.get(x+jx, y+jy) == Tile::Empty && !jumping_targets.contains(&(x+jx, y+jy)) && !result.contains(&(x+jx, y+jy)) {
                        jumping_targets.push((x+jx, y+jy));
                    }
                }
            }

            result.push((x, y));
        }

        result.retain(|&pos| pos != (x, y));
        result
    }

    fn move_piece(&mut self, (fx, fy): (i8, i8), (tx, ty): (i8, i8)) {
        if !self.is_valid_location(fx, fy) || !self.is_valid_location(tx, ty) {
            panic!("Invalid locations for move_piece");
        }

        let from = self.get(fx, fy);
        self.set(tx, ty, from);
        self.set(fx, fy, Tile::Empty);
        self.current_player = 3-self.current_player;
    }
}

impl Game {
    fn move_piece(&mut self, (fx, fy): (i8, i8), (tx, ty): (i8, i8)) {
        if !self.state.is_valid_location(fx, fy) || !self.state.is_valid_location(tx, ty) {
            panic!("Invalid locations for move_piece");
        }

        self.moves.push(Move { from: (fx, fy), to: (tx, ty) });
        self.state.move_piece((fx, fy), (tx, ty));
    }

    fn undo(&mut self) {
        if let Some(mov) = self.moves.pop() {
            let (fx, fy) = mov.from;
            let (tx, ty) = mov.to;
            let from = self.state.get(tx, ty);
            self.state.set(fx, fy, from);
            self.state.set(tx, ty, Tile::Empty);
            self.state.current_player = 3-self.state.current_player;
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        let mut state = GameState {
            board: [[Tile::Invalid; BOARD_HEIGHT as usize]; BOARD_WIDTH as usize],
            current_player: 1,
        };

        for y in 0..BOARD_HEIGHT as i8 {
            for x in 0..BOARD_WIDTH as i8 {
                if !state.is_valid_location(x, y) {
                    continue;
                }
                if y < 5 && (x-6).abs() < 3 {
                    state.set(x, y, Tile::Player(1));
                } else if y > 11  && (x-6).abs() < 3 {
                    state.set(x, y, Tile::Player(2));
                } else {
                    state.set(x, y, Tile::Empty);
                }
            }
        }

        state
    }
}

fn draw_board(canvas: &mut sdl2::render::WindowCanvas, state: &GameState) {
    for y in 0..BOARD_HEIGHT as i8 {
        for x in 0..BOARD_WIDTH as i8 {
            let tile = state.get(x, y);
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            tile.draw(canvas, x, y);
        }
    }
}

fn board_space_to_screen_space(x: i8, y: i8) -> (i32, i32) {
    let screen_y = 120 + y as i32*20;
    let screen_x = if y % 2 == 0 {
        260 + x as i32 * 20
    } else {
        260 + x as i32 * 20 - 10
    };

    (screen_x, screen_y)
}

fn nearest_board_position(state: &GameState, x: i32, y: i32) -> Option<(i8, i8)> {
    fn dist(x: i32, y: i32, x2: i32, y2: i32) -> f32 {
        ((x-x2).pow(2) as f32 + (y-y2).pow(2) as f32).sqrt()
    }

    let mut min_x = None;
    let mut min_y = None;
    let mut min_d = None;

    for by in 0..BOARD_HEIGHT as i8 {
        for bx in 0..BOARD_WIDTH as i8 {
            if state.get(bx, by) == Tile::Invalid {
                continue;
            }

            let (sx, sy) = board_space_to_screen_space(bx, by);

            let d = dist(x, y, sx, sy);
            if min_d == None || d < min_d.unwrap() {
                min_x = Some(bx);
                min_y = Some(by);
                min_d = Some(d);
            }
        }
    }

    if min_d == None || min_d.unwrap() > 15.0 {
        None
    } else {
        Some((min_x.unwrap(), min_y.unwrap()))
    }
}


fn main() {
    let sdl = sdl2::init().unwrap();
    let video = sdl.video().unwrap();
    let window = video.window("halma", 800, 600).position_centered().build().unwrap();
    let mut canvas = window.into_canvas().software().build().unwrap();

    canvas.set_draw_color(Color::RGB(224, 224, 224));
    canvas.clear();
    canvas.present();

    let mut game: Game = Default::default();
    let mut mouse_x = 0;
    let mut mouse_y = 0;
    let mut selection = None;
    let mut display_moves = false;

    let mut events = sdl.event_pump().unwrap();
    'mainloop: loop {
        canvas.set_draw_color(Color::RGB(224, 224, 224));
        canvas.clear();

        for event in events.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'mainloop,
                Event::KeyDown { keycode: Some(Keycode::R), .. } => game = Default::default(),
                Event::KeyDown { keycode: Some(Keycode::M), .. } => display_moves = !display_moves,
                Event::KeyDown { keycode: Some(Keycode::U), .. } => game.undo(),
                Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                    let ai = AI::new(game.state);
                    let mov = ai.calculate_move(2);
                    game.move_piece(mov.from, mov.to);
                }
                Event::MouseMotion { x, y, .. } => {
                    mouse_x = x;
                    mouse_y = y;
                },
                Event::MouseButtonDown { x: mouse_x, y: mouse_y, .. } => {
                    match selection {
                        None => {
                            if let Some((x, y)) = nearest_board_position(&game.state, mouse_x, mouse_y) {
                                let tile = game.state.get(x, y);
                                if tile == Tile::Player(game.state.current_player) {
                                    selection = Some((x, y));
                                } else {
                                    selection = None;
                                }
                            }
                        }
                        Some((x, y)) => {
                            if let Some((bx, by)) = nearest_board_position(&game.state, mouse_x, mouse_y) {
                                if game.state.reachable_from(x, y).contains(&(bx, by)) {
                                    game.move_piece((x, y), (bx, by));
                                }
                            }

                            selection = None;
                        }
                    }
                }
                _ => {}
            }
        }

        draw_board(&mut canvas, &game.state);

        if let Some((x, y)) = nearest_board_position(&game.state, mouse_x, mouse_y) {
            let tile = game.state.get(x, y);
            if tile == Tile::Player(game.state.current_player) {
                let (screen_x, screen_y) = board_space_to_screen_space(x, y);
                canvas.set_draw_color(player_color(game.state.current_player));
                canvas.draw_rect(sdl2::rect::Rect::new(screen_x-6, screen_y-6, 12, 12)).unwrap();
            }
        }

        if let Some((x, y)) = selection {
            if let Tile::Player(pid) = game.state.get(x, y) {
                let (screen_x, screen_y) = board_space_to_screen_space(x, y);
                canvas.set_draw_color(player_color(pid));
                canvas.draw_rect(sdl2::rect::Rect::new(screen_x-6, screen_y-6, 12, 12)).unwrap();
            }

            for (rx, ry) in game.state.reachable_from(x, y) {
                let (screen_x, screen_y) = board_space_to_screen_space(rx, ry);
                canvas.set_draw_color(Color::RGB(0, 0, 0));
                canvas.draw_rect(sdl2::rect::Rect::new(screen_x-6, screen_y-6, 12, 12)).unwrap();
            }
        }

        if display_moves {
            let ai = AI::new(game.state);
            let moves = ai.possible_moves();
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            for &mov in &moves {
                let (fx, fy) = mov.from;
                let (tx, ty) = mov.to;
                canvas.draw_line(board_space_to_screen_space(fx, fy), board_space_to_screen_space(tx, ty)).unwrap();
            }

            canvas.string(32, 8, &format!("Possible moves: {}", &moves.len()), Color::RGB(0, 0, 0)).unwrap();
        }

        canvas.set_draw_color(player_color(game.state.current_player));
        canvas.fill_rect(Some(sdl2::rect::Rect::new(0, 0, 24, 24))).unwrap();

        canvas.present();
        ::std::thread::sleep(::std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }
}
