use crate::logger::Logger;
use crate::matcher::Matcher;
use crate::target::Target;
use niri_ipc::{Action, Request, Response};
use niri_ipc::socket::Socket;
use regex::Regex;

#[derive(Clone)]
pub struct Window {
    pub id: u64,
    pub app_id: Option<String>,
    pub title: Option<String>,
}

impl Target for Window {
    type Id = u64;

    fn id(&self) -> Self::Id {
        self.id
    }
}

pub struct WindowMatcher {
    app_id_regexes: Vec<Regex>,
    title_regexes: Vec<Regex>,
    target_ids: Vec<u64>,
}

impl WindowMatcher {
    pub fn new(
        app_id_regexes: Vec<Regex>,
        title_regexes: Vec<Regex>,
        target_ids: Vec<u64>,
    ) -> Self {
        WindowMatcher {
            app_id_regexes,
            title_regexes,
            target_ids,
        }
    }
}

impl Matcher<Window> for WindowMatcher {
    fn matches(&self, window: &Window) -> Option<String> {
        if self.target_ids.contains(&window.id) {
            return Some("id".to_string());
        }

        if let Some(app_id) = &window.app_id {
            for regex in &self.app_id_regexes {
                if regex.is_match(app_id) {
                    return Some("app_id".to_string());
                }
            }
        }

        if let Some(title) = &window.title {
            for regex in &self.title_regexes {
                if regex.is_match(title) {
                    return Some("title".to_string());
                }
            }
        }

        None
    }
}

pub type WindowState = crate::target::TargetState<Window>;

pub fn populate_window_cache(
    socket: &mut Socket,
    state: &mut WindowState,
    logger: &dyn Logger<Window>,
) -> Result<(), Box<dyn std::error::Error>> {
    let reply = socket.send(Request::Windows)?;
    if let Ok(Response::Windows(win_list)) = reply {
        for window in win_list {
            let app_id = window.app_id.clone();
            let title = window.title.clone();
            state.targets.insert(window.id, Window {
                id: window.id,
                app_id: app_id.clone(),
                title: title.clone(),
            });
            logger.log_target_loaded(&Window {
                id: window.id,
                app_id,
                title,
            });
        }
    }
    Ok(())
}

pub fn send_set_dynamic_cast_window(window_id: u64) -> Result<(), Box<dyn std::error::Error>> {
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
