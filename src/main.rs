extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Tile {
    Empty,
    Invalid,
}

const BOARD_WIDTH: u8 = 13;
const BOARD_HEIGHT: u8 = 19;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Board {
    board: [[Tile; BOARD_HEIGHT as usize]; BOARD_WIDTH as usize],
}

impl Board {
    fn is_valid_location(&self, x: i8, y: i8) -> bool {
        let width = match y {
            0...5 => y+1,
            5...10 => 21-y,
            10...14 => y+1,
            14...18 => 21-y,
            _ => 0,
        };

        if y % 2 == 0 {
            (6-x).abs() < width/2
        } else {
            (6-x).abs() < width/2 && (7-x).abs() < width/2
        }
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
}

impl Default for Board {
    fn default() -> Self {
        let mut board = Board {
            board: [[Tile::Invalid; BOARD_HEIGHT as usize]; BOARD_WIDTH as usize],
        };

        for y in 0..BOARD_HEIGHT as i8 {
            for x in 0..BOARD_WIDTH as i8 {
                if board.is_valid_location(x, y) {
                    board.set(x, y, Tile::Empty);
                }
            }
        }

        board
    }
}

fn draw_board(canvas: &mut sdl2::render::WindowCanvas, board: &Board) {
    for y in 0..BOARD_HEIGHT as i8 {
        for x in 0..BOARD_WIDTH as i8 {
            let tile = board.get(x, y);
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            if tile == Tile::Empty {
                let (draw_x, draw_y) = board_space_to_screen_space(x, y);
                canvas.fill_rect(Some(sdl2::rect::Rect::new(draw_x-4, draw_y-4, 8, 8))).unwrap();
            }
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

fn nearest_board_position(board: &Board, x: i32, y: i32) -> Option<(i8, i8)> {
    fn dist(x: i32, y: i32, x2: i32, y2: i32) -> f32 {
        ((x-x2).pow(2) as f32 + (y-y2).pow(2) as f32).sqrt()
    }

    let mut min_x = None;
    let mut min_y = None;
    let mut min_d = None;

    for by in 0..BOARD_HEIGHT as i8 {
        for bx in 0..BOARD_WIDTH as i8 {
            if board.get(bx, by) == Tile::Invalid {
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

    let board: Board = Default::default();
    let mut mouse_x = 0;
    let mut mouse_y = 0;

    let mut events = sdl.event_pump().unwrap();
    'mainloop: loop {
        canvas.set_draw_color(Color::RGB(224, 224, 224));
        canvas.clear();

        for event in events.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'mainloop,
                Event::MouseMotion { x, y, .. } => {
                    mouse_x = x;
                    mouse_y = y;
                },
                _ => {}
            }
        }

        draw_board(&mut canvas, &board);

        if let Some((x, y)) = nearest_board_position(&board, mouse_x, mouse_y) {
            let (screen_x, screen_y) = board_space_to_screen_space(x, y);
            canvas.set_draw_color(Color::RGB(255, 0, 0));
            canvas.draw_rect(sdl2::rect::Rect::new(screen_x-5, screen_y-5, 10, 10)).unwrap();
        }

        canvas.present();
        ::std::thread::sleep(::std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }
}
