use crate::logger::Logger;
use crate::matcher::Matcher;
use crate::target::Target;
use regex::Regex;

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

pub type OutputState = crate::target::TargetState<Output>;

pub fn populate_output_cache(
    _socket: &mut niri_ipc::socket::Socket,
    state: &mut OutputState,
    logger: &dyn Logger<Output>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = (state, logger);
    Ok(())
}

pub fn send_set_dynamic_cast_output(_output_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
