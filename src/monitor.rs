use crate::logger::Logger;
use crate::matcher::Matcher;
use crate::target::Target;
use regex::Regex;

#[derive(Clone)]
pub struct Monitor {
    pub id: u64,
    pub name: Option<String>,
    pub description: Option<String>,
}

impl Target for Monitor {
    type Id = u64;

    fn id(&self) -> Self::Id {
        self.id
    }
}

pub struct MonitorMatcher {
    name_regexes: Vec<Regex>,
    description_regexes: Vec<Regex>,
    target_ids: Vec<u64>,
}

impl MonitorMatcher {
    pub fn new(
        name_regexes: Vec<Regex>,
        description_regexes: Vec<Regex>,
        target_ids: Vec<u64>,
    ) -> Self {
        MonitorMatcher {
            name_regexes,
            description_regexes,
            target_ids,
        }
    }
}

impl Matcher<Monitor> for MonitorMatcher {
    fn matches(&self, monitor: &Monitor) -> Option<String> {
        if self.target_ids.contains(&monitor.id) {
            return Some("id".to_string());
        }

        if let Some(name) = &monitor.name {
            for regex in &self.name_regexes {
                if regex.is_match(name) {
                    return Some("name".to_string());
                }
            }
        }

        if let Some(description) = &monitor.description {
            for regex in &self.description_regexes {
                if regex.is_match(description) {
                    return Some("description".to_string());
                }
            }
        }

        None
    }
}

pub type MonitorState = crate::target::TargetState<Monitor>;

pub fn populate_monitor_cache(
    state: &mut MonitorState,
    monitors: Vec<Monitor>,
    logger: &impl Logger<Monitor>,
) -> Result<(), Box<dyn std::error::Error>> {
    for monitor in monitors {
        state.targets.insert(monitor.id, monitor.clone());
        logger.log_target_loaded(&monitor);
    }
    Ok(())
}

pub fn send_set_dynamic_cast_monitor(_monitor_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
