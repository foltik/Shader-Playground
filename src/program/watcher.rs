use std::{path::Path, sync::mpsc::Sender};


pub fn spawn(file: &Path, tx: Sender<()>) {
    let file = file.to_owned();
    std::thread::spawn(move || {
        let (notify_tx, notify_rx) = std::sync::mpsc::channel();

        let mut watcher = notify::raw_watcher(notify_tx).expect("Failed to initialize filesystem watcher!");

        use notify::Watcher;
        watcher
            .watch(&file.parent().unwrap(), notify::RecursiveMode::Recursive)
            .unwrap();

        loop {
            let ev = notify_rx.recv().unwrap();
            if let (Some(path), Ok(op)) = (ev.path, ev.op) {
                // Only notify the compiler on CLOSE_WRITE, since WRITE
                // can happen before the full file is written.
                if path == file && op == notify::op::CLOSE_WRITE {
                    tx.send(()).unwrap();
                }
            }
        }
    });
}
