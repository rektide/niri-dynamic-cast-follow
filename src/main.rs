use clap::Parser;
use niri_ipc::socket::Socket;
use niri_ipc::{Event, Request, Response};
use regex::Regex;
use serde_json;

mod follower;
mod logger;
mod matcher;
mod output;
mod target;
mod window;
use follower::Follower;
use logger::Logger;

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

    /// Output name patterns to match (regex supported)
    #[arg(long, short = 'o')]
    output: Vec<String>,

    /// Specific output IDs to match
    #[arg(long, short = 'O')]
    output_id: Vec<u64>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// JSON output mode (implies verbose)
    #[arg(long)]
    json: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let has_window_flags = !cli.app_id.is_empty() || !cli.title.is_empty() || !cli.id.is_empty();
    let has_output_flags = !cli.output.is_empty() || !cli.output_id.is_empty();

    if has_window_flags && has_output_flags {
        eprintln!("Error: cannot mix window and output matching criteria");
        eprintln!("Use either window flags (--app-id, --title, --id) OR output flags (--output, --output-id)");
        std::process::exit(1);
    }

    if !has_window_flags && !has_output_flags {
        eprintln!("Error: at least one matching criterion must be provided");
        eprintln!("Window: --app-id, --title, or --id");
        eprintln!("Output: --output or --output-id");
        std::process::exit(1);
    }

    let json = cli.json;
    let verbose = cli.verbose;

    let logger = logger::GenericLogger::new(verbose, json);

    let mut socket = Socket::connect()?;

    if has_window_flags {
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

        let state = window::WindowState::new();

        let follower = Follower::new(state, Box::new(matcher), Box::new(logger), json);

        follower.run(
            socket,
            window::populate_window_cache,
            handle_window_event,
        )?
    } else {
        let matcher = output::OutputMatcher::new(
            cli.output
                .iter()
                .map(|pattern| Regex::new(pattern).expect("Invalid regex"))
                .collect(),
            cli.output_id,
        );

        let mut state = output::OutputState::new();

        <logger::GenericLogger as Logger<output::Output>>::log_connected(&logger);

        let outputs = Vec::new();
        output::populate_output_cache(&mut state, outputs, &logger)?;

        let reply = socket.send(Request::EventStream)?;
        if !matches!(reply, Ok(Response::Handled)) {
            if json {
                eprintln!(
                    "{}",
                    serde_json::json!({"event": "error", "message": "Failed to start event stream"})
                );
            } else {
                eprintln!("Failed to start event stream: {:?}", reply);
            }
            std::process::exit(1);
        }

        <logger::GenericLogger as Logger<output::Output>>::log_streaming(&logger);

        let mut read_event = socket.read_events();

        loop {
            match read_event() {
                Ok(event) => {
                    if let Err(e) = handle_output_event(event, &mut state, &matcher, &logger, json)
                    {
                        if json {
                            eprintln!(
                                "{}",
                                serde_json::json!({"event": "error", "message": e.to_string()})
                            );
                        } else {
                            eprintln!("Error handling event: {}", e);
                        }
                    }
                }
                Err(e) => {
                    if json {
                        eprintln!(
                            "{}",
                            serde_json::json!({"event": "error", "message": e.to_string()})
                        );
                    } else {
                        eprintln!("Error reading event: {}", e);
                    }
                    break;
                }
            }
        }
    }

    Ok(())
}

fn handle_window_event(
    event: Event,
    state: &mut window::WindowState,
    matcher: &dyn matcher::Matcher<window::Window>,
    logger: &dyn logger::Logger<window::Window>,
    _json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        Event::WindowOpenedOrChanged { window } => {
            logger.log_target_changed(&window::Window {
                id: window.id,
                app_id: window.app_id.clone(),
                title: window.title.clone(),
            });
            state.targets.insert(
                window.id,
                window::Window {
                    id: window.id,
                    app_id: window.app_id,
                    title: window.title,
                },
            );
        }
        Event::WindowFocusChanged { id } => {
            let new_window_id = id;
            let window = new_window_id.and_then(|wid| state.targets.get(&wid));
            logger.log_focus_change(new_window_id, window);

            if state.current_focused_id == new_window_id {
                return Ok(());
            }

            state.current_focused_id = new_window_id;

            if let Some(window_id) = new_window_id {
                if let Some(window) = state.targets.get(&window_id) {
                    if let Some(mt) = matcher.matches(window) {
                        logger.log_target_matched(window, &mt);
                        window::send_set_dynamic_cast_window(window_id)?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_output_event(
    _event: Event,
    _state: &mut output::OutputState,
    _matcher: &dyn matcher::Matcher<output::Output>,
    _logger: &dyn logger::Logger<output::Output>,
    _json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
