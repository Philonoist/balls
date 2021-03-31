// extern crate sdl2;
// use legion::world::SubWorld;
// use legion::IntoQuery;
// use legion::*;
// use sdl2::{gfx::primitives::DrawRenderer, pixels::Color};

// use crate::ball::Ball;

// pub struct Graphics {
//     pub sdl_context: sdl2::Sdl,
//     pub canvas: sdl2::render::Canvas<sdl2::video::Window>,
// }

// pub struct DisplayConfig {
//     pub width: u32,
//     pub height: u32,
// }

// pub fn init_graphics(display_config: DisplayConfig) -> Graphics {
//     let sdl_context = sdl2::init().unwrap();
//     let video_subsystem = sdl_context.video().unwrap();

//     let window = video_subsystem
//         .window("Balls", display_config.width, display_config.height)
//         .position_centered()
//         .build()
//         .unwrap();

//     Graphics {
//         sdl_context: sdl_context,
//         canvas: window.into_canvas().build().unwrap(),
//     }
// }

// #[system]
// #[read_component(Ball)]
// pub fn render_balls(world: &mut SubWorld, #[resource] graphics: &mut Graphics) {
//     for ball in <&Ball>::query().iter(world) {
//         let irad = ball.radius as i16;
//         graphics
//             .canvas
//             .filled_circle(
//                 (0.5 + ball.position[0]) as i16,
//                 (0.5 + ball.position[1]) as i16,
//                 irad,
//                 Color::RGB(0, 0, 255),
//             )
//             .expect("ok");
//     }
//     graphics.canvas.present();
//     graphics.canvas.set_draw_color(Color::RGB(0, 0, 0));
//     graphics.canvas.clear();
// }
