use clap::Parser;
use niri_ipc::{Event, Request, Response};
use niri_ipc::socket::Socket;
use regex::Regex;
use serde_json;

mod target;
mod window;
use window::Window;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Window app-id patterns to match (regex supported)
    #[arg(long, short = 'a')]
    app_id: Vec<String>,

    /// Window title patterns to match (regex supported)
    #[arg(long, short = 't')]
    title: Vec<String>,

    /// Specific window IDs to match
    #[arg(long, short = 'i')]
    id: Vec<u64>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// JSON output mode (implies verbose)
    #[arg(long)]
    json: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.app_id.is_empty() && cli.title.is_empty() && cli.id.is_empty() {
        eprintln!("Error: at least one matching criterion must be provided");
        eprintln!("Use --app-id, --title, or --id");
        std::process::exit(1);
    }

    let matcher = window::WindowMatcher::new(
        cli.app_id
            .iter()
            .map(|pattern| Regex::new(pattern).expect("Invalid regex"))
            .collect(),
        cli.title
            .iter()
            .map(|pattern| Regex::new(pattern).expect("Invalid regex"))
            .collect(),
        cli.id,
    );

    let json = cli.json;
    let verbose = cli.verbose;

    let mut state = window::WindowState::new();

    let logger = Logger::new(verbose, json);

    let mut socket = Socket::connect()?;
    logger.log_connected();

    window::populate_window_cache(&mut socket, &mut state, &logger)?;

    let reply = socket.send(Request::EventStream)?;
    if !matches!(reply, Ok(Response::Handled)) {
        if json {
            eprintln!("{}", serde_json::json!({"event": "error", "message": "Failed to start event stream"}));
        } else {
            eprintln!("Failed to start event stream: {:?}", reply);
        }
        std::process::exit(1);
    }

    logger.log_streaming();

    let mut read_event = socket.read_events();

    loop {
        match read_event() {
            Ok(event) => {
                if let Err(e) = handle_event(
                    event,
                    &mut state,
                    &matcher,
                    &logger,
                ) {
                    if json {
                        eprintln!("{}", serde_json::json!({"event": "error", "message": e.to_string()}));
                    } else {
                        eprintln!("Error handling event: {}", e);
                    }
                }
            }
            Err(e) => {
                if json {
                    eprintln!("{}", serde_json::json!({"event": "error", "message": e.to_string()}));
                } else {
                    eprintln!("Error reading event: {}", e);
                }
                break;
            }
        }
    }

    Ok(())
}

fn handle_event(
    event: Event,
    state: &mut window::WindowState,
    matcher: &window::WindowMatcher,
    logger: &Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        Event::WindowOpenedOrChanged { window } => {
            logger.log_window_changed(&window::Window {
                id: window.id,
                app_id: window.app_id.clone(),
                title: window.title.clone(),
            });
            state.windows.insert(window.id, window::Window {
                id: window.id,
                app_id: window.app_id,
                title: window.title,
            });
        }
        Event::WindowFocusChanged { id } => {
            let new_window_id = id;
            let window = new_window_id.and_then(|wid| state.windows.get(&wid));
            logger.log_focus_change(new_window_id, window);

            if state.current_focused_window_id == new_window_id {
                return Ok(());
            }

            state.current_focused_window_id = new_window_id;

            if let Some(window_id) = new_window_id {
                if let Some(window) = state.windows.get(&window_id) {
                    if let Some(mt) = matcher.matches(window) {
                        logger.log_window_matched(window, &mt);
                        window::send_set_dynamic_cast_window(window_id)?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

struct Logger {
    verbose: bool,
    json: bool,
}

impl Logger {
    fn new(verbose: bool, json: bool) -> Self {
        Logger { verbose, json }
    }

    fn json_verbose(&self) -> bool {
        self.json && self.verbose
    }

    fn log_connected(&self) {
        if self.json_verbose() {
            println!("{}", serde_json::json!({"event": "connected"}));
        } else if self.verbose {
            eprintln!("Connected to niri IPC socket");
        }
    }

    fn log_window_loaded(&self, window: &Window) {
        if self.json_verbose() {
            println!("{}", serde_json::json!({
                "event": "window-loaded",
                "id": window.id,
                "app_id": window.app_id,
                "title": window.title
            }));
        } else if self.verbose {
            eprintln!("Window loaded: id={}, app_id={:?}, title={:?}", window.id, window.app_id, window.title);
        }
    }

    fn log_streaming(&self) {
        if self.json_verbose() {
            println!("{}", serde_json::json!({"event": "streaming"}));
        } else if self.verbose {
            eprintln!("Event stream started");
        }
    }

    fn log_window_changed(&self, window: &Window) {
        if self.json_verbose() {
            println!("{}", serde_json::json!({
                "event": "window-changed",
                "id": window.id,
                "title": window.title,
                "app_id": window.app_id
            }));
        } else if self.verbose {
            eprintln!(
                "Window changed: id={}, title={:?}, app_id={:?}",
                window.id,
                window.title,
                window.app_id
            );
        }
    }

    fn log_focus_change(&self, window_id: Option<u64>, window: Option<&Window>) {
        if self.json_verbose() {
            if let Some(id) = window_id {
                if let Some(w) = window {
                    println!("{}", serde_json::json!({
                        "event": "focus-change",
                        "id": id,
                        "title": w.title,
                        "app_id": w.app_id
                    }));
                } else {
                    println!("{}", serde_json::json!({
                        "event": "focus-change",
                        "id": id,
                        "title": null,
                        "app_id": null
                    }));
                }
            } else {
                println!("{}", serde_json::json!({
                    "event": "focus-change",
                    "id": null
                }));
            }
        } else if self.verbose {
            let id_str = window_id.map(|i| i.to_string()).unwrap_or_else(|| "None".to_string());
            if let Some(_id) = window_id {
                if let Some(w) = window {
                    eprintln!("Window focus changed: id={}, title={:?}, app_id={:?}", id_str, w.title, w.app_id);
                } else {
                    eprintln!("Window focus changed: {} (window info not available yet)", id_str);
                }
            } else {
                eprintln!("Window focus changed: {}", id_str);
            }
        }
    }

    fn log_window_matched(&self, window: &Window, match_type: &str) {
        if self.json {
            println!("{}", serde_json::json!({
                "event": "window-matched",
                "match_type": match_type,
                "id": window.id,
                "app_id": window.app_id,
                "title": window.title
            }));
        } else if self.verbose {
            eprintln!(
                "Window matched! match_type={}, id={}, app_id={:?}, title={:?}",
                match_type, window.id, window.app_id, window.title
            );
        }
    }
}