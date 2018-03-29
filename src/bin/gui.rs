extern crate halma;
extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::gfx::primitives::DrawRenderer;

use halma::*;
use halma::ai::AI;

fn draw_tile(tile: Tile, canvas: &mut sdl2::render::WindowCanvas, board_x: i8, board_y: i8) {
    let (draw_x, draw_y) = board_space_to_screen_space(board_x, board_y);

    match tile {
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

fn player_color(id: u8) -> Color {
    match id {
        0 => Color::RGB(255, 0, 0),
        1 => Color::RGB(0, 0, 255),
        _ => unimplemented!()
    }
}


fn draw_board(canvas: &mut sdl2::render::WindowCanvas, state: &GameState) {
    for y in 0..BOARD_HEIGHT as i8 {
        for x in 0..BOARD_WIDTH as i8 {
            let tile = state.get(x, y);
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            draw_tile(tile, canvas, x, y);
        }
    }
}

fn board_space_to_screen_space(x: i8, y: i8) -> (i32, i32) {
    let screen_y = 10 + y as i32*20;
    let screen_x = if y % 2 == 0 {
        20 + x as i32 * 20
    } else {
        20 + x as i32 * 20 - 10
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
    let window = video.window("halma", 280, 340).position_centered().build().unwrap();
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
    let depth = 8;

    let mut ai0 = AI::new(*game.state());
    ai0.print_statistics = true;
    let mut ai1 = AI::new(*game.state());

    let mut autoplay0 = true;
    let mut autoplay1 = true;

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
                    let mov;
                    if game.state().current_player() == 0 {
                        mov = ai0.calculate_move(depth);
                    } else {
                        mov = ai1.calculate_move(depth);
                    }
                    game.move_piece(mov);
                    ai0.make_move(mov);
                    ai1.make_move(mov);
                }
                Event::MouseMotion { x, y, .. } => {
                    mouse_x = x;
                    mouse_y = y;
                },
                Event::MouseButtonDown { x: mouse_x, y: mouse_y, .. } => {
                    match selection {
                        None => {
                            if let Some((x, y)) = nearest_board_position(game.state(), mouse_x, mouse_y) {
                                let tile = game.state().get(x, y);
                                if tile == Tile::Player(game.state().current_player()) {
                                    selection = Some((x, y));
                                } else {
                                    selection = None;
                                }
                            }
                        }
                        Some((x, y)) => {
                            if let Some((bx, by)) = nearest_board_position(game.state(), mouse_x, mouse_y) {
                                if game.state().moves_from(x, y).contains(&Move { from: (x, y), to: (bx, by) }) {
                                    let mov = Move { from: (x, y), to: (bx, by) };
                                    game.move_piece(mov);
                                    ai0.make_move(mov);
                                    ai1.make_move(mov);
                                }
                            }

                            selection = None;
                        }
                    }
                }
                _ => {}
            }
        }
        
        if autoplay0 && game.state().current_player() == 0 {
            let mov = ai0.calculate_move(depth);
            game.move_piece(mov);
            ai0.make_move(mov);
            ai1.make_move(mov);
        } else if autoplay1 && game.state().current_player() == 1 {
            let mov = ai1.calculate_move(depth);
            game.move_piece(mov);
            ai0.make_move(mov);
            ai1.make_move(mov);
        }

        draw_board(&mut canvas, game.state());

        if let Some((x, y)) = nearest_board_position(game.state(), mouse_x, mouse_y) {
            let tile = game.state().get(x, y);
            if tile == Tile::Player(game.state().current_player()) {
                let (screen_x, screen_y) = board_space_to_screen_space(x, y);
                canvas.set_draw_color(player_color(game.state().current_player()));
                canvas.draw_rect(sdl2::rect::Rect::new(screen_x-6, screen_y-6, 12, 12)).unwrap();
            }
        }

        if let Some((x, y)) = selection {
            if let Tile::Player(pid) = game.state().get(x, y) {
                let (screen_x, screen_y) = board_space_to_screen_space(x, y);
                canvas.set_draw_color(player_color(pid));
                canvas.draw_rect(sdl2::rect::Rect::new(screen_x-6, screen_y-6, 12, 12)).unwrap();
            }

            for Move { from: _, to: (rx, ry) } in game.state().moves_from(x, y) {
                let (screen_x, screen_y) = board_space_to_screen_space(rx, ry);
                canvas.set_draw_color(Color::RGB(0, 0, 0));
                canvas.draw_rect(sdl2::rect::Rect::new(screen_x-6, screen_y-6, 12, 12)).unwrap();
            }
        }

        if display_moves {
            let ai = AI::new(*game.state());
            let moves = ai.possible_moves();
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            for &mov in &moves {
                let (fx, fy) = mov.from;
                let (tx, ty) = mov.to;
                canvas.draw_line(board_space_to_screen_space(fx, fy), board_space_to_screen_space(tx, ty)).unwrap();
            }

            canvas.string(32, 8, &format!("Possible moves: {}", &moves.len()), Color::RGB(0, 0, 0)).unwrap();
        }

        if let Some(&last_move) = game.last_move() {
            canvas.set_draw_color(Color::RGB(168, 168, 168));
            let (fx, fy) = last_move.from;
            let (tx, ty) = last_move.to;
            let (screen_fx, screen_fy) = board_space_to_screen_space(fx, fy);
            let (screen_tx, screen_ty) = board_space_to_screen_space(tx, ty);
            canvas.draw_rect(sdl2::rect::Rect::new(screen_fx-8, screen_fy-8, 16, 16)).unwrap();
            canvas.draw_rect(sdl2::rect::Rect::new(screen_tx-8, screen_ty-8, 16, 16)).unwrap();
        }

        canvas.set_draw_color(player_color(game.state().current_player()));
        canvas.fill_rect(Some(sdl2::rect::Rect::new(0, 0, 24, 24))).unwrap();

        canvas.present();
        ::std::thread::sleep(::std::time::Duration::new(0, 1_000_000_000u32 / 60));

        if game.state().won(0) {
            autoplay0 = false;
            autoplay1 = false;
        } else if game.state().won(1) {
            autoplay0 = false;
            autoplay1 = false;
        }
    }
}
