use std::{fs::File, io::Write, path::PathBuf};

use secs_codegen::config::Config;

pub fn main() {
    let config = Config::new("ecs/main.ron")
        .components("ecs/components.ron")
        .resources("ecs/resources.ron")
        .systems("ecs/systems.ron");

    let out = secs_codegen::build(config);
    let out_path = PathBuf::from("src/ecs.rs");

    let mut file = File::create(out_path).expect("Failed to open/create the output file");
    file.write_all(out.as_bytes())
        .expect("Failed to write codegen output");
}
