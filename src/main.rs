mod app;
mod build_info;
mod document;
mod highlight;

use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
};
use document::Document;
use std::{
    error::Error,
    io::{self, IsTerminal, Read},
    time::Duration,
};

const HELP: &str = "mwt-jv — interactive JSON terminal viewer

Usage: mwt-jv <file.json>
       mwt-jv -                 Read JSON from stdin
       mwt-jv --demo            Open the built-in example
       cat file.json | mwt-jv

Keys: ↑↓ / j k navigate · Space fold · t table · Esc back · ? help · Ctrl+Q quit
Line numbers refer to pretty-printed JSON. Nesting starts at depth 0.";

fn load() -> Result<Option<(Document, String)>, Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() > 1 {
        return Err(format!("Expected one input file.\n\n{HELP}").into());
    }
    let arg = args.first().map(|s| s.to_string_lossy());
    match arg.as_deref() {
        Some("--help" | "-h") => {
            println!("{HELP}");
            return Ok(None);
        }
        Some("--version" | "-V") => {
            println!(
                "mwt-jv {} · built {} ({})",
                build_info::VERSION,
                build_info::TIMESTAMP,
                build_info::age()
            );
            return Ok(None);
        }
        None if io::stdin().is_terminal() => {
            println!("{HELP}");
            return Ok(None);
        }
        _ => {}
    }
    let (text, source) = match arg.as_deref() {
        Some("--demo") => (
            include_str!("../examples/demo.json").to_owned(),
            "demo.json".into(),
        ),
        Some("-") | None => {
            let mut text = String::new();
            io::stdin().read_to_string(&mut text)?;
            (text, "stdin".into())
        }
        Some(_) => {
            let path = &args[0];
            let text = std::fs::read_to_string(path)
                .map_err(|e| format!("Cannot read {}: {e}", path.to_string_lossy()))?;
            (text, path.to_string_lossy().into_owned())
        }
    };
    let value =
        serde_json::from_str(&text).map_err(|e| format!("Invalid JSON in {source}: {e}"))?;
    Ok(Some((Document::new(value), source)))
}

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        ratatui::restore();
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let Some((doc, source)) = load()? else {
        return Ok(());
    };
    if !io::stdout().is_terminal() {
        return Err("Interactive viewer requires a terminal on stdout".into());
    }
    let mut app = App::new(doc, source);
    let mut terminal = ratatui::try_init()?;
    let _guard = TerminalGuard;
    execute!(io::stdout(), EnableMouseCapture)?;
    loop {
        terminal.draw(|frame| app.draw(frame))?;
        // Refresh the relative build age even while no keys are pressed.
        if !event::poll(Duration::from_secs(1))? {
            continue;
        }
        let was_table = app.table.is_some();
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if app.key(key) {
                    break;
                }
            }
            Event::Mouse(mouse) => app.mouse(mouse),
            _ => {}
        }
        if was_table != app.table.is_some() {
            // Clear both the physical screen and Ratatui's diff buffers so the
            // next mode is painted completely, without remnants of the old view.
            terminal.clear()?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("mwt-jv: {error}");
        std::process::exit(1);
    }
}
