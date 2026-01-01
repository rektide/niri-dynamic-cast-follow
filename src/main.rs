use clap::Parser;
use niri_ipc::{Action, Event, Request, Response};
use niri_ipc::socket::Socket;
use regex::Regex;
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

    let mut socket = Socket::connect()?;
    if cli.verbose {
        eprintln!("Connected to niri IPC socket");
    }

    let reply = socket.send(Request::EventStream)?;
    if !matches!(reply, Ok(Response::Handled)) {
        eprintln!("Failed to start event stream: {:?}", reply);
        std::process::exit(1);
    }

    if cli.verbose {
        eprintln!("Event stream started");
    }

    let mut read_event = socket.read_events();
    let mut windows: HashMap<u64, (Option<String>, Option<String>)> = HashMap::new();
    let mut current_focused_window_id: Option<u64> = None;

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
                    cli.verbose,
                ) {
                    eprintln!("Error handling event: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error reading event: {}", e);
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
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        Event::WindowOpenedOrChanged { window } => {
            if verbose {
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
            if verbose {
                eprintln!("Window focus changed: {:?}", id);
            }

            let new_window_id = id;
            
            if *current_focused_window_id == new_window_id {
                return Ok(());
            }

            *current_focused_window_id = new_window_id;

            if let Some(window_id) = new_window_id {
                if let Some((app_id, title)) = windows.get(&window_id) {
                    let matches = window_matches(
                        window_id,
                        app_id,
                        title,
                        app_id_regexes,
                        title_regexes,
                        target_ids,
                    );

                    if matches {
                        if verbose {
                            eprintln!(
                                "Window matched! Triggering SetDynamicCastWindow for id={}",
                                window_id
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
) -> bool {
    if target_ids.contains(&window_id) {
        return true;
    }

    if let Some(app_id) = app_id {
        for regex in app_id_regexes {
            if regex.is_match(app_id) {
                return true;
            }
        }
    }

    if let Some(title) = title {
        for regex in title_regexes {
            if regex.is_match(title) {
                return true;
            }
        }
    }

    false
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
