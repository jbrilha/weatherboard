use embedded_graphics::prelude::*;
use embedded_graphics_simulator::{
    OutputSettings, SimulatorDisplay, SimulatorEvent, Window, sdl2::Keycode,
};
use ui::TriColor;

fn main() {
    let mut epd = SimulatorDisplay::with_default_color(Size::new(400, 300), TriColor::White);
    let mut window = Window::new(
        "weatherboard",
        &OutputSettings {
            scale: 2,
            ..Default::default()
        },
    );

    let mut round = 0;

    'running: loop {
        ui::test_pattern(&mut epd, "simulator", round).unwrap();
        window.update(&epd);

        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'running,
                SimulatorEvent::KeyDown {
                    keycode: Keycode::Space,
                    ..
                } => round += 1,
                _ => {}
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
