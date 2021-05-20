use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

mod program;
mod render;


fn main() {
    env_logger::Builder::from_default_env()
        .parse_filters("shaderview=info")
        .init();

    // Parse args
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: shadervis [fragment.glsl]");
        std::process::exit(1);
    }
    let file = std::fs::canonicalize(std::path::PathBuf::from(&args[1])).unwrap();

    if !file.exists() {
        println!("File does not exist!");
        std::process::exit(1);
    }

    // Create channels for message passing between threads
    let (watch_tx, watch_rx) = std::sync::mpsc::channel();
    let (pipeline_tx, pipeline_rx) = std::sync::mpsc::channel();

    // Initialize the platform event loop
    let event_loop = EventLoop::new();

    // Spawn workers
    let mut renderer = render::Renderer::new(&event_loop, pipeline_rx);
    program::watcher::spawn(&file, watch_tx);
    program::compiler::spawn(Arc::clone(&renderer.device), &file, watch_rx, pipeline_tx);

    // Kick off the main event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::MainEventsCleared => renderer.update(),
            Event::RedrawRequested(_) => renderer.render(),
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => renderer.resize(size),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }

        renderer.event(&event);
    });
}
