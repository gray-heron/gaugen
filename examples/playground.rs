extern crate gaugen;

fn main() {
    gaugen::session::SessionBuilder::new()
        .register_components(gaugen::basic_components::components())
        .register_components(gaugen::geometry_components::components())
        .init(|session: &mut gaugen::session::Session| {
            loop {
                let mut view = session
                    .new_view("resources/playground.json")
                    .expect("Failed to initialize view from screen.json");

                let mut hooks = gaugen::Hooks::new();

                gaugen::add_hook(&mut hooks, "my_textfield", "text", "Bye world!".to_string());
                gaugen::add_hook(
                    &mut hooks,
                    "my_textfield",
                    "front_color",
                    "ffffffff".to_string(),
                );
                gaugen::add_hook(&mut hooks, "my_indicator", "value", 99);

                if !session.draw(&mut view, &gaugen::frontend::DarkPalette {}, &hooks) {
                    break; // handle window being closed, etc.
                }
            }
        });
}
