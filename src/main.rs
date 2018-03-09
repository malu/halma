extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;

fn main() {
    let sdl = sdl2::init().unwrap();
    let video = sdl.video().unwrap();
    let window = video.window("halma", 800, 600).position_centered().build().unwrap();
    let mut canvas = window.into_canvas().software().build().unwrap();

    canvas.set_draw_color(Color::RGB(224, 224, 224));
    canvas.clear();
    canvas.present();

    let mut events = sdl.event_pump().unwrap();
    'mainloop: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'mainloop,
                _ => {}
            }
        }

        canvas.present();
        ::std::thread::sleep(::std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }
}
