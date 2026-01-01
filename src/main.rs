use clap::Parser;
use niri_ipc::{Action, Event, Request, Response};
use niri_ipc::socket::Socket;
use regex::Regex;
use serde_json;
use std::collections::HashMap;

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

    let app_id_regexes: Vec<Regex> = cli
        .app_id
        .iter()
        .map(|pattern| Regex::new(pattern).expect("Invalid regex"))
        .collect();

    let title_regexes: Vec<Regex> = cli
        .title
        .iter()
        .map(|pattern| Regex::new(pattern).expect("Invalid regex"))
        .collect();

    let target_ids: Vec<u64> = cli.id;

    let json = cli.json;
    let verbose = cli.verbose;
    let json_verbose = json && verbose;

    let mut socket = Socket::connect()?;
    if json_verbose {
        println!("{}", serde_json::json!({"event": "connected"}));
    } else if verbose {
        eprintln!("Connected to niri IPC socket");
    }

    let mut windows: HashMap<u64, (Option<String>, Option<String>)> = HashMap::new();
    let mut current_focused_window_id: Option<u64> = None;

    // Get current window list to populate cache
    let reply = socket.send(Request::Windows)?;
    if let Ok(Response::Windows(win_list)) = reply {
        for window in win_list {
            windows.insert(window.id, (window.app_id.clone(), window.title.clone()));
            if json_verbose {
                println!("{}", serde_json::json!({
                    "event": "window-loaded",
                    "id": window.id,
                    "app_id": window.app_id,
                    "title": window.title
                }));
            } else if verbose {
                eprintln!("Window loaded: id={}, app_id={:?}, title={:?}", window.id, window.app_id, window.title);
            }
        }
    }

    let reply = socket.send(Request::EventStream)?;
    if !matches!(reply, Ok(Response::Handled)) {
        if json {
            eprintln!("{}", serde_json::json!({"event": "error", "message": "Failed to start event stream"}));
        } else {
            eprintln!("Failed to start event stream: {:?}", reply);
        }
        std::process::exit(1);
    }

    if json_verbose {
        println!("{}", serde_json::json!({"event": "streaming"}));
    } else if verbose {
        eprintln!("Event stream started");
    }

    let mut read_event = socket.read_events();

    loop {
        match read_event() {
            Ok(event) => {
                if let Err(e) = handle_event(
                    event,
                    &mut windows,
                    &mut current_focused_window_id,
                    &app_id_regexes,
                    &title_regexes,
                    &target_ids,
                    verbose,
                    json,
                    json_verbose,
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
    windows: &mut HashMap<u64, (Option<String>, Option<String>)>,
    current_focused_window_id: &mut Option<u64>,
    app_id_regexes: &[Regex],
    title_regexes: &[Regex],
    target_ids: &[u64],
    verbose: bool,
    json: bool,
    json_verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        Event::WindowOpenedOrChanged { window } => {
            if json_verbose {
                println!("{}", serde_json::json!({
                    "event": "window-changed",
                    "id": window.id,
                    "title": window.title,
                    "app_id": window.app_id
                }));
            } else if verbose {
                eprintln!(
                    "Window changed: id={}, title={:?}, app_id={:?}",
                    window.id,
                    window.title,
                    window.app_id
                );
            }
            windows.insert(window.id, (window.app_id, window.title));
        }
        Event::WindowFocusChanged { id } => {
            let new_window_id = id;

            if json_verbose {
                if let Some(window_id) = new_window_id {
                    if let Some((app_id, title)) = windows.get(&window_id) {
                        println!("{}", serde_json::json!({
                            "event": "focus-change",
                            "id": window_id,
                            "title": title,
                            "app_id": app_id
                        }));
                    } else {
                        println!("{}", serde_json::json!({
                            "event": "focus-change",
                            "id": window_id,
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
            } else if verbose {
                let id_str = id.map(|i| i.to_string()).unwrap_or_else(|| "None".to_string());
                if let Some(window_id) = new_window_id {
                    if let Some((app_id, title)) = windows.get(&window_id) {
                        eprintln!("Window focus changed: id={}, title={:?}, app_id={:?}", id_str, title, app_id);
                    } else {
                        eprintln!("Window focus changed: {} (window info not available yet)", id_str);
                    }
                } else {
                    eprintln!("Window focus changed: {}", id_str);
                }
            }

            if *current_focused_window_id == new_window_id {
                return Ok(());
            }

            *current_focused_window_id = new_window_id;

            if let Some(window_id) = new_window_id {
                if let Some((app_id, title)) = windows.get(&window_id) {
                    let match_type = window_matches(
                        window_id,
                        app_id,
                        title,
                        app_id_regexes,
                        title_regexes,
                        target_ids,
                    );

                    if let Some(mt) = match_type {
                        if json {
                            println!("{}", serde_json::json!({
                                "event": "window-matched",
                                "match_type": mt,
                                "id": window_id,
                                "app_id": app_id,
                                "title": title
                            }));
                        } else if verbose {
                            eprintln!(
                                "Window matched! match_type={}, id={}, app_id={:?}, title={:?}",
                                mt, window_id, app_id, title
                            );
                        }
                        send_set_dynamic_cast_window(window_id)?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn window_matches(
    window_id: u64,
    app_id: &Option<String>,
    title: &Option<String>,
    app_id_regexes: &[Regex],
    title_regexes: &[Regex],
    target_ids: &[u64],
) -> Option<String> {
    if target_ids.contains(&window_id) {
        return Some("id".to_string());
    }

    if let Some(app_id) = app_id {
        for regex in app_id_regexes {
            if regex.is_match(app_id) {
                return Some("app_id".to_string());
            }
        }
    }

    if let Some(title) = title {
        for regex in title_regexes {
            if regex.is_match(title) {
                return Some("title".to_string());
            }
        }
    }

    None
}

fn send_set_dynamic_cast_window(window_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let mut action_socket = Socket::connect()?;
    let action = Action::SetDynamicCastWindow { id: Some(window_id) };
    let request = Request::Action(action);

    let reply = action_socket.send(request)?;

    match reply {
        Ok(Response::Handled) => Ok(()),
        Ok(resp) => {
            eprintln!("Unexpected response: {:?}", resp);
            Ok(())
        }
        Err(e) => {
            eprintln!("Error sending action: {}", e);
            Err(e.into())
        }
    }
}
