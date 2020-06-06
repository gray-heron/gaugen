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
use std::net;

use serde_json::json;
use std::collections::HashMap;

fn bytes_to_u32_le(array: &[u8]) -> u32 {
    assert!(array.len() >= 4);

    ((array[0] as u32) << 0)
        + ((array[1] as u32) << 8)
        + ((array[2] as u32) << 16)
        + ((array[3] as u32) << 24)
}

const XPLANE_UNIT_SIZE: usize = 4;
const XPLANE_UNITS_PER_PACKET: usize = 8;

fn listen(socket: &net::UdpSocket) -> HashMap<(u32, u32), f32> {
    let mut buffer = [0; 1000];
    let mut ret = HashMap::new();

    let (number_of_bytes, _) = socket.recv_from(&mut buffer).expect("no data received");
    assert!(buffer[0] == ('D' as u8));
    assert!(buffer[1] == ('A' as u8));
    assert!(buffer[2] == ('T' as u8));
    assert!(buffer[3] == ('A' as u8));

    let mut cursor = 5;

    loop {
        let id = buffer[cursor] as u32;
        cursor += XPLANE_UNIT_SIZE;

        for i in 0..XPLANE_UNITS_PER_PACKET {
            let v = f32::from_bits(bytes_to_u32_le(&buffer[cursor..cursor + XPLANE_UNIT_SIZE]));

            ret.insert((id, i as u32), v);

            cursor += XPLANE_UNIT_SIZE;
        }

        if cursor >= number_of_bytes {
            break;
        }
    }

    ret
}

fn add_trivial_hook<T>(hooks: &mut gaugen::Hooks, component: &str, property: &str, value: T)
where
    T: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
{
    if hooks.contains_key(&component.to_string()) {
        hooks
            .get_mut(&component.to_string())
            .unwrap()
            .insert(property.to_string(), serde_json::json!(value));
    } else {
        let mut properties = serde_json::Map::new();
        properties.insert(property.to_string(), serde_json::json!(value));
        hooks.insert(component.to_string(), properties);
    }
}

fn init_host(host: &str) -> net::UdpSocket {
    println!("initializing host");
    let socket = net::UdpSocket::bind(host).expect("failed to bind host socket");

    socket
}

fn main() {
    let mut gilrs = Gilrs::new().unwrap();

    // TODO(alex): Currently hangs on listening, there must be a way to set a timeout, simply
    // setting the timeout to true did not work.
    let socket = init_host("127.0.0.1:6112");
    let message = String::from("hello");
    let msg_bytes = message.into_bytes();

    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let mut running = true;
    let mut prev_time = 0.0;

    //let mut active_gamepad = None;

    SessionBuilder::new()
        .register_components(basic_components::components())
        .register_components(geometry_components::components())
        .init(|session: &mut Session| loop { 
            let mut view = session.new_view("screen.json");

            let telemetry = listen(&socket);
            let mut hooks = HashMap::new();

            add_trivial_hook(&mut hooks, "speed", "value", telemetry[&(3, 0)]);
            add_trivial_hook(&mut hooks, "flaps", "value", telemetry[&(13, 4)] * 40.0);
            add_trivial_hook(&mut hooks, "alt", "value", telemetry[&(20, 5)] / 1000.0);

            add_trivial_hook(&mut hooks, "gear", "front_color",
                match telemetry[&(67, 0)] < 0.01 {
                    true =>  "ff808080".to_string(),
                    false => "ff000000".to_string(),
                }
            );

            add_trivial_hook(&mut hooks, "gear", "back_color",
                match (telemetry[&(67, 0)] < 0.01, telemetry[&(67, 0)] > 0.99) {
                    (true, false) => "ff000000".to_string(),
                    (false, true) => "ff008f00".to_string(),
                    _ => "ff8f0000".to_string(),
                }
            );
            
            add_trivial_hook(
                &mut hooks,
                "ssi",
                "pitch",
                telemetry[&(17, 0)] / 180.0 * 3.14,
            );
            add_trivial_hook(
                &mut hooks,
                "ssi",
                "roll",
                telemetry[&(17, 1)] / 180.0 * 3.14,
            );
            add_trivial_hook(&mut hooks, "ssi", "yaw", telemetry[&(17, 2)] / 180.0 * 3.14);

            add_trivial_hook(&mut hooks, "e11", "value", telemetry[&(34, 0)] / 1000.0);
            add_trivial_hook(&mut hooks, "e21", "value", telemetry[&(34, 1)] / 1000.0);
            add_trivial_hook(&mut hooks, "e12", "value", telemetry[&(46, 0)]);
            add_trivial_hook(&mut hooks, "e22", "value", telemetry[&(46, 1)]);
            add_trivial_hook(&mut hooks, "e13", "value", telemetry[&(37, 0)]);
            add_trivial_hook(&mut hooks, "e23", "value", telemetry[&(37, 1)]);
            add_trivial_hook(&mut hooks, "e14", "value", telemetry[&(45, 0)]);
            add_trivial_hook(&mut hooks, "e24", "value", telemetry[&(45, 1)]);

            match view {
                Some(ref mut view) => {
                    if !session.draw(view, &frontend::DarkPalette {}, &hooks) {
                        break;
                    }
                }
                _ => {}
            };
        });
}
