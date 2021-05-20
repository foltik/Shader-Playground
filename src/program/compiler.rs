use std::{
    borrow::Cow,
    fs,
    path::Path,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
};

use crate::program::Program;

pub fn spawn(
    device: Arc<wgpu::Device>,
    file: &std::path::Path,
    watch_rx: Receiver<()>,
    program_tx: Sender<Program>,
) {
    let file = file.to_owned();
    std::thread::spawn(move || {
        let mut compiler = shaderc::Compiler::new().expect("Failed to create shader compiler!");

        // Create the vertex shader module.
        let vertex = compile_literal(
            &device,
            &mut compiler,
            shaderc::ShaderKind::Vertex,
            "vertex.glsl",
            include_str!("../vertex.glsl"),
        )
        .expect("Failed to compile vertex shader!");

        loop {
            // Compile the file, create a program from it, and send it to the renderer.
            match compile(&device, &mut compiler, shaderc::ShaderKind::Fragment, &file) {
                Ok((fragment, reflect)) => {
                    let program = Program::new(Arc::clone(&device), &vertex, fragment, reflect);

                    match program {
                        Ok(program) => program_tx.send(program).unwrap(),
                        Err(e) => log::error!("Error:\n {}", e),
                    };
                }
                Err(shaderc::Error::CompilationError(_, err)) => log::error!("Error:\n {}", err),
                Err(err) => log::error!("{:?}", err),
            }

            // Wait for a message from the watcher indicating that
            // the file has changed and we should compile it again.
            let _ = watch_rx.recv().unwrap();
        }
    });
}

// Compile a shader from a file into a wgpu shader module along with its reflection data.
fn compile(
    device: &wgpu::Device,
    compiler: &mut shaderc::Compiler,
    kind: shaderc::ShaderKind,
    file: &Path,
) -> Result<(wgpu::ShaderModule, spirv_reflect::ShaderModule), shaderc::Error> {
    let dir = file.parent().unwrap();

    // Configure the compiler to try to resolve includes in the same folder
    // as the file being compiled
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_include_callback(move |file, _ty, _source, _depth| {
        let inc = dir.join(Path::new(file));
        let abs = std::fs::canonicalize(&inc).unwrap();

        let content = fs::read_to_string(&inc).map_err(|e| e.to_string())?;

        Ok(shaderc::ResolvedInclude {
            resolved_name: abs.to_str().unwrap().to_string(),
            content,
        })
    });

    let content = std::fs::read_to_string(file).expect("Failed to read shader file!");
    let spirv = compiler.compile_into_spirv(&content, kind, "", "main", Some(&options))?;

    let reflect_mod = spirv_reflect::create_shader_module(spirv.as_binary_u8())
        .expect("Failed to reflect shader module!");

    let wgpu_mod = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::SpirV(Cow::Borrowed(spirv.as_binary())),
        flags: wgpu::ShaderFlags::empty(),
    });

    Ok((wgpu_mod, reflect_mod))
}

// Compile a shader from source in a string into a wgpu shader module.
fn compile_literal(
    device: &wgpu::Device,
    compiler: &mut shaderc::Compiler,
    kind: shaderc::ShaderKind,
    name: &str,
    content: &str,
) -> Result<wgpu::ShaderModule, shaderc::Error> {
    let spirv = compiler.compile_into_spirv(&content, kind, name, "main", None)?;

    Ok(device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::SpirV(Cow::Borrowed(spirv.as_binary())),
        flags: wgpu::ShaderFlags::empty(),
    }))
}
