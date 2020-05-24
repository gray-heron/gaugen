mod basic_components;
mod frontend;
mod gaugen;
mod geometry_components;
mod rtps;
mod session;

extern crate gl;
extern crate glutin;
extern crate nalgebra as na;
extern crate nanovg;
extern crate rand;

use gilrs::{Axis, Button, Event, Gilrs};
use nalgebra::Vector2;
use session::*;

use std::time::Instant;

use serde_json::json;
use std::collections::HashMap;

fn get_elapsed(instant: &Instant) -> f32 {
    let elapsed = instant.elapsed();
    let elapsed = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9;
    elapsed as f32
}

fn main() {
    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let start_time = Instant::now();
    let mut running = true;
    let mut prev_time = 0.0;

    //let mut active_gamepad = None;

    SessionBuilder::new()
        .register_components(basic_components::components())
        .register_components(geometry_components::components())
        .init(|session: &mut Session| {
            let mut view = session.new_view("screen.json").unwrap();

            while session.draw(&mut view, &frontend::DarkPalette {}, &HashMap::new()) {}
        });
}
