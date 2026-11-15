#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! Native editor entry point; intentionally separate from the compiler CLI.
use laml_ui::app;

fn main() -> eframe::Result {
    let mut args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "serve") {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::System::Console::AttachConsole(
                windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
            );
        }
        if let Err(error) = laml_ui::server::run(&args[1..]) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "--host-worker") {
        if !(3..=4).contains(&args.len()) {
            std::process::exit(1);
        }
        let limits = args.get(3).is_some_and(|arg| arg == "--limits");
        if laml_ui::server::worker_main(args[1].clone().into(), args[2].clone().into(), limits)
            .is_err()
        {
            std::process::exit(1);
        }
        return Ok(());
    }

    if args.first().is_some_and(|arg| arg == "--version") {
        println!("laml-ui {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let capture = if args.first().is_some_and(|arg| arg == "--capture") {
        if args.len() < 2 {
            eprintln!("--capture requires a new PNG output path");
            std::process::exit(2);
        }
        let output = std::path::PathBuf::from(args.remove(1));
        args.remove(0);
        Some(output)
    } else {
        None
    };
    let capture_view = if args.first().is_some_and(|arg| arg == "--capture-view") {
        if capture.is_none() || args.len() < 2 {
            eprintln!("--capture-view requires --capture and general, style, add-node or icons");
            std::process::exit(2);
        }
        args.remove(0);
        let view = args.remove(0).to_string_lossy().into_owned();
        if !matches!(view.as_str(), "general" | "style" | "add-node" | "icons") {
            eprintln!("Unknown capture view");
            std::process::exit(2);
        }
        Some(view)
    } else {
        None
    };
    if args.len() > 1 || args.first().is_some_and(|arg| arg == "--help") {
        println!(
            "usage: laml-ui [--capture NEW.png [--capture-view general|style|add-node|icons]] [FILE.laml]"
        );
        return Ok(());
    }
    // Startup is a native host adapter; the portable file state receives only
    // resolved backend addresses and performs no canonicalization itself.
    let path = args.first().map(|arg| match std::fs::canonicalize(arg) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{}: {error}", std::path::Path::new(arg).display());
            std::process::exit(1);
        }
    });
    let source = match &path {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("{}: {error}", path.display());
                std::process::exit(1);
            }
        },
        None => "semantics {\n    A\n    B\n    A -> B\n}\n".into(),
    };
    let capturing = capture.is_some();
    let success = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let captured = success.clone();
    let result = eframe::run_native(
        "LAML Editor",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1568.0, 975.0])
                .with_min_inner_size([1000.0, 700.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            app::Editor::install_appearance(&cc.egui_ctx);
            let source = if path.is_none() {
                app::Editor::new_source(&cc.egui_ctx)
            } else {
                source
            };
            let empty = path.is_none() && !capturing;
            let mut editor = app::Editor::new(source, path);
            editor.set_capture(capture, success);
            editor.set_capture_view(capture_view);
            Ok(Box::new(if empty {
                app::Workspace::empty(editor)
            } else {
                app::Workspace::new(editor)
            }))
        }),
    );
    if capturing && !captured.load(std::sync::atomic::Ordering::Acquire) {
        std::process::exit(1);
    }
    result
}
