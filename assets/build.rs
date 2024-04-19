use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rerun-if-changed=shaders");

    let compiler = shaderc::Compiler::new().unwrap();

    std::fs::read_dir("shaders/")
        .unwrap()
        .filter_map(|path| path.ok())
        .filter_map(|path| path.file_name().into_string().ok().map(PathBuf::from))
        .filter(|path| {
            path.extension()
                .map(|extension| extension == "glsl")
                .unwrap_or_default()
        })
        .for_each(|path| {
            println!("cargo::warning=Compiling {}", path.display());

            let kind = match path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .split(".")
                .collect::<Vec<&str>>()
                .into_iter()
                .rev()
                .skip(1)
                .next()
                .unwrap()
            {
                "vert" => shaderc::ShaderKind::Vertex,
                "frag" => shaderc::ShaderKind::Fragment,
                kind => panic!("Unknown shader kind: {kind}"),
            };
            let source = std::fs::read_to_string(Path::new("shaders/").join(&path)).unwrap();
            let compiled = compiler
                .compile_into_spirv(
                    &source,
                    kind,
                    &path.clone().display().to_string(),
                    "main",
                    None,
                )
                .unwrap();
            std::fs::write(
                Path::new("shaders/").join(path.with_extension("spv")),
                bytemuck::cast_slice::<u32, u8>(&compiled.as_binary()),
            )
            .unwrap();
        });
}
