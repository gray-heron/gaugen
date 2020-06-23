extern crate gaugen;

extern crate glutin;
extern crate nalgebra as na;
extern crate nanovg;
extern crate rand;

use std::collections::HashMap;
use std::net;
use std::time::Duration;

fn bytes_to_u32(array: &[u8]) -> u32 {
    assert!(array.len() >= 4);

    ((array[0] as u32) << 0)
        + ((array[1] as u32) << 8)
        + ((array[2] as u32) << 16)
        + ((array[3] as u32) << 24)
}

const XPLANE_UNIT_SIZE: usize = 4;
const XPLANE_UNITS_PER_PACKET: usize = 8;

fn listen(socket: &net::UdpSocket) -> Option<HashMap<(u32, u32), f32>> {
    let mut buffer = [0; 1000];
    let mut ret = HashMap::new();

    let (number_of_bytes, _) = socket.recv_from(&mut buffer).ok()?;
    assert!(buffer[0] == ('D' as u8));
    assert!(buffer[1] == ('A' as u8));
    assert!(buffer[2] == ('T' as u8));
    assert!(buffer[3] == ('A' as u8));

    let mut cursor = 5;

    loop {
        let id = buffer[cursor] as u32;
        cursor += XPLANE_UNIT_SIZE;

        for i in 0..XPLANE_UNITS_PER_PACKET {
            let v = f32::from_bits(bytes_to_u32(&buffer[cursor..cursor + XPLANE_UNIT_SIZE]));

            ret.insert((id, i as u32), v);

            cursor += XPLANE_UNIT_SIZE;
        }

        if cursor >= number_of_bytes {
            break;
        }
    }

    Some(ret)
}

fn md71_telemetry_to_hooks(telemetry: HashMap<(u32, u32), f32>) -> gaugen::Hooks {
    let mut hooks = HashMap::new();

    gaugen::add_hook(&mut hooks, "speed", "value", telemetry[&(3, 0)]);
    gaugen::add_hook(&mut hooks, "flaps", "value", telemetry[&(13, 4)] * 40.0);
    gaugen::add_hook(&mut hooks, "alt", "value", telemetry[&(20, 5)] / 1000.0);

    gaugen::add_hook(
        &mut hooks,
        "gear",
        "front_color",
        match telemetry[&(67, 0)] < 0.01 {
            true => "ff808080".to_string(),
            false => "ff000000".to_string(),
        },
    );

    gaugen::add_hook(
        &mut hooks,
        "gear",
        "back_color",
        match (telemetry[&(67, 0)] < 0.01, telemetry[&(67, 0)] > 0.99) {
            (true, false) => "ff000000".to_string(),
            (false, true) => "ff008f00".to_string(),
            _ => "ff8f0000".to_string(),
        },
    );

    gaugen::add_hook(
        &mut hooks,
        "ssi",
        "pitch",
        telemetry[&(17, 0)] / 180.0 * 3.14,
    );
    gaugen::add_hook(
        &mut hooks,
        "ssi",
        "roll",
        telemetry[&(17, 1)] / 180.0 * 3.14,
    );
    gaugen::add_hook(&mut hooks, "ssi", "yaw", telemetry[&(17, 2)] / 180.0 * 3.14);

    gaugen::add_hook(&mut hooks, "e11", "value", telemetry[&(34, 0)] / 1000.0);
    gaugen::add_hook(&mut hooks, "e21", "value", telemetry[&(34, 1)] / 1000.0);
    gaugen::add_hook(&mut hooks, "e12", "value", telemetry[&(46, 0)]);
    gaugen::add_hook(&mut hooks, "e22", "value", telemetry[&(46, 1)]);
    gaugen::add_hook(&mut hooks, "e13", "value", telemetry[&(37, 0)]);
    gaugen::add_hook(&mut hooks, "e23", "value", telemetry[&(37, 1)]);
    gaugen::add_hook(&mut hooks, "e14", "value", telemetry[&(45, 0)]);
    gaugen::add_hook(&mut hooks, "e24", "value", telemetry[&(45, 1)]);

    hooks
}

fn main() {
    let socket = net::UdpSocket::bind("127.0.0.1:6112").expect("failed to bind host socket");
    socket
        .set_read_timeout(Some(Duration::new(1, 0)))
        .expect("failed set socket read timeout");

    gaugen::session::SessionBuilder::new()
        .register_components(gaugen::basic_components::components())
        .register_components(gaugen::geometry_components::components())
        .init(|session: &mut gaugen::session::Session| {
            let mut view = session.new_view("resources/screen_xplane11.json");
            loop {
                let hooks = match listen(&socket) {
                    Some(telemetry) => md71_telemetry_to_hooks(telemetry),
                    None => HashMap::new(),
                };

                match view {
                    Some(ref mut view) => {
                        if !session.draw(view, &gaugen::frontend::DarkPalette {}, &hooks) {
                            break;
                        }
                    }
                    _ => {}
                };
            }
        });
}
