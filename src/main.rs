use clap::Parser;
use niri_ipc::socket::Socket;
use niri_ipc::Event;
use regex::Regex;

mod follower;
mod logger;
mod matcher;
mod output;
mod target;
mod window;
use follower::Follower;

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

    let socket = Socket::connect()?;

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

        let state = output::OutputState::new();

        let follower = Follower::new(state, Box::new(matcher), Box::new(logger), json);

        follower.run(
            socket,
            output::populate_output_cache,
            handle_output_event,
        )?
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
    event: Event,
    state: &mut output::OutputState,
    matcher: &dyn matcher::Matcher<output::Output>,
    logger: &dyn logger::Logger<output::Output>,
    _json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        Event::WindowOpenedOrChanged { window } => {
            // Track window-to-workspace mapping
            if let Some(workspace_id) = window.workspace_id {
                state.update_window_workspace(window.id, workspace_id);
            }
        }
        Event::WorkspacesChanged { workspaces } => {
            // Update workspace-to-output mappings and track focused workspace
            for workspace in &workspaces {
                if let Some(output_name) = &workspace.output {
                    let output_id = output::get_output_id_from_name(output_name);
                    state.update_workspace_output(workspace.id, output_id);

                    if workspace.is_focused {
                        // Focused workspace means this output is now active
                        handle_output_focused(output_id, state, matcher, logger)?;
                    }
                }
            }
        }
        Event::WindowFocusChanged { id } => {
            // Window focus change may indicate output focus change
            if let Some(window_id) = id {
                if let Some(workspace_id) = state.get_workspace_for_window(window_id) {
                    if let Some(output_id) = state.get_output_for_workspace(workspace_id) {
                        handle_output_focused(output_id, state, matcher, logger)?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_output_focused(
    output_id: u64,
    state: &mut output::OutputState,
    matcher: &dyn matcher::Matcher<output::Output>,
    logger: &dyn logger::Logger<output::Output>,
) -> Result<(), Box<dyn std::error::Error>> {
    logger.log_focus_change(Some(output_id), state.targets.get(&output_id));

    if state.current_focused_id == Some(output_id) {
        return Ok(());
    }

    state.current_focused_id = Some(output_id);

    if let Some(output) = state.targets.get(&output_id) {
        if let Some(mt) = matcher.matches(output) {
            logger.log_target_matched(output, &mt);
            output::send_set_dynamic_cast_output(output_id)?;
        }
    }
    Ok(())
}