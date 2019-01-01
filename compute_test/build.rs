extern crate shaderc;

use std::error::Error;

fn main() -> Result<(), Box<Error>> {
    // Tell the build script to only run again if we change our source shaders
    println!("cargo:rerun-if-changed=source_assets/shaders");

    // Create destination path if necessary
    std::fs::create_dir_all("assets/gen/shaders")?;

    for entry in std::fs::read_dir("source_assets/shaders")? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            let in_path = entry.path();

            let shader_kind =
                in_path
                    .extension()
                    .and_then(|ext| match ext.to_string_lossy().as_ref() {
                        "vert" => Some(shaderc::ShaderKind::Vertex),
                        "frag" => Some(shaderc::ShaderKind::Fragment),
                        "compute" => Some(shaderc::ShaderKind::Compute),
                        _ => None,
                    });

            if let Some(shader_kind) = shader_kind {
                let source = std::fs::read_to_string(&in_path)?;
                let mut compiler = shaderc::Compiler::new().unwrap();
                let mut options = shaderc::CompileOptions::new().unwrap();
                let shader_name = in_path.file_name().unwrap().to_string_lossy();
                let mut binary_result = compiler
                    .compile_into_spirv(&source, shader_kind, &shader_name, "main", Some(&options))
                    .unwrap();
                let out_path = format!("assets/gen/shaders/{}.spv", shader_name);
                std::fs::write(&out_path, &binary_result.as_binary_u8())?;
            }
        }
    }

    Ok(())
}
