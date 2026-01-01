use crate::logger::Logger;
use crate::matcher::Matcher;
use crate::target::Target;
use regex::Regex;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct Output {
    pub id: u64,
    pub name: Option<String>,
}

impl Target for Output {
    type Id = u64;

    fn id(&self) -> Self::Id {
        self.id
    }
}

pub struct OutputMatcher {
    name_regexes: Vec<Regex>,
    target_ids: Vec<u64>,
}

impl OutputMatcher {
    pub fn new(
        name_regexes: Vec<Regex>,
        target_ids: Vec<u64>,
    ) -> Self {
        OutputMatcher {
            name_regexes,
            target_ids,
        }
    }
}

impl Matcher<Output> for OutputMatcher {
    fn matches(&self, output: &Output) -> Option<String> {
        if self.target_ids.contains(&output.id) {
            return Some("id".to_string());
        }

        if let Some(name) = &output.name {
            for regex in &self.name_regexes {
                if regex.is_match(name) {
                    return Some("name".to_string());
                }
            }
        }

        None
    }
}

pub struct OutputState {
    pub targets: HashMap<u64, Output>,
    pub current_focused_id: Option<u64>,
    workspace_to_output: HashMap<u64, u64>,
    window_to_workspace: HashMap<u64, u64>,
}

impl OutputState {
    pub fn new() -> Self {
        OutputState {
            targets: HashMap::new(),
            current_focused_id: None,
            workspace_to_output: HashMap::new(),
            window_to_workspace: HashMap::new(),
        }
    }

    pub fn update_workspace_output(&mut self, workspace_id: u64, output_id: u64) {
        self.workspace_to_output.insert(workspace_id, output_id);
    }

    pub fn update_window_workspace(&mut self, window_id: u64, workspace_id: u64) {
        self.window_to_workspace.insert(window_id, workspace_id);
    }

    pub fn remove_window(&mut self, window_id: u64) {
        self.window_to_workspace.remove(&window_id);
    }

    pub fn get_output_for_workspace(&self, workspace_id: u64) -> Option<u64> {
        self.workspace_to_output.get(&workspace_id).copied()
    }

    pub fn get_workspace_for_window(&self, window_id: u64) -> Option<u64> {
        self.window_to_workspace.get(&window_id).copied()
    }
}

impl crate::target::FollowerState for OutputState {
    type Target = Output;
    type Id = u64;

    fn get_current_focused_id(&self) -> Option<Self::Id> {
        self.current_focused_id
    }

    fn set_current_focused_id(&mut self, id: Option<Self::Id>) {
        self.current_focused_id = id;
    }

    fn get_targets(&self) -> &HashMap<Self::Id, Self::Target> {
        &self.targets
    }

    fn get_targets_mut(&mut self) -> &mut HashMap<Self::Id, Self::Target> {
        &mut self.targets
    }
}

pub fn get_output_id_from_name(name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

pub fn populate_output_cache(
    socket: &mut niri_ipc::socket::Socket,
    state: &mut OutputState,
    logger: &dyn Logger<Output>,
) -> Result<(), Box<dyn std::error::Error>> {
    let reply = socket.send(niri_ipc::Request::Outputs)?;
    if let Ok(niri_ipc::Response::Outputs(outputs_map)) = reply {
        for (name, _niri_output) in outputs_map {
            let output_id = get_output_id_from_name(&name);
            let output = Output {
                id: output_id,
                name: Some(name.clone()),
            };
            state.targets.insert(output.id, output.clone());
            logger.log_target_loaded(&output);
        }
    }

    let windows_reply = socket.send(niri_ipc::Request::Windows)?;
    if let Ok(niri_ipc::Response::Windows(windows)) = windows_reply {
        for window in &windows {
            if let Some(workspace_id) = window.workspace_id {
                state.update_window_workspace(window.id, workspace_id);
            }
        }
    }

    Ok(())
}

pub fn send_set_dynamic_cast_output(_output_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
