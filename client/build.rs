#![feature(option_result_contains)]

use std::{io::Write, path::Path, path::PathBuf};

fn main() {
    let files_in_shader_dir: Vec<PathBuf> = std::fs::read_dir("./shaders")
        .unwrap()
        .map_while(|path| path.ok())
        .filter(|path| path.path().is_file())
        .map(|path| path.path())
        .collect();

    let shader_paths = files_in_shader_dir.iter().filter(|path| {
        path.extension().contains(&"wgsl") && !path.file_name().contains(&"common.wgsl")
    });

    let common_shader_path = files_in_shader_dir
        .iter()
        .find(|path| path.file_name().contains(&"common.wgsl"))
        .expect("Missing common.wgsl");

    let common_shader_buf = std::fs::read(common_shader_path).unwrap();

    for shader_path in shader_paths {
        print!("cargo:rerun-if-changed={}", shader_path.to_string_lossy());

        let shader_buf = std::fs::read(&shader_path).unwrap();

        let shader_output_path = Path::new("../data").join(shader_path);

        let mut output_file = std::fs::File::create(shader_output_path).unwrap();
        output_file.write(&common_shader_buf[..]).unwrap();
        output_file.write(&shader_buf[..]).unwrap();
    }
}
