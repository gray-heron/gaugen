use std::fs;
use filetime::FileTime;

extern crate gaugen;

fn main() {
    gaugen::session::SessionBuilder::new()
        .register_components(gaugen::basic_components::components())
        .register_components(gaugen::geometry_components::components())
        .init(|session: &mut gaugen::session::Session| {
            let get_modtime = || {
                let metadata = fs::metadata("resources/playground.json").unwrap();
                let mtime = FileTime::from_last_modification_time(&metadata);
                mtime.seconds()
            };

            let mk_view = |session: &mut gaugen::session::Session| { session
                .new_view("resources/playground.json")
            };

            let mut view = mk_view(session).unwrap();
            let mut last_modtime = get_modtime();
                
            loop {
                let modtime = get_modtime();

                if last_modtime != modtime {
                    view = match mk_view(session){
                        Some(view) => view,
                        None => continue
                    };
                    
                    last_modtime = modtime;
                }

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
