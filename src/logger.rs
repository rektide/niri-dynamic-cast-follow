use serde_json;
use crate::monitor::Monitor;
use crate::window::Window;

pub trait Logger<T> {
    fn log_connected(&self);
    fn log_streaming(&self);
    fn log_target_loaded(&self, target: &T);
    fn log_target_changed(&self, target: &T);
    fn log_focus_change(&self, target_id: Option<u64>, target: Option<&T>);
    fn log_target_matched(&self, target: &T, match_type: &str);
}

pub struct GenericLogger {
    verbose: bool,
    json: bool,
}

impl GenericLogger {
    pub fn new(verbose: bool, json: bool) -> Self {
        GenericLogger { verbose, json }
    }

    fn json_verbose(&self) -> bool {
        self.json && self.verbose
    }
}

impl Logger<Window> for GenericLogger {
    fn log_connected(&self) {
        if self.json_verbose() {
            println!("{}", serde_json::json!({"event": "connected"}));
        } else if self.verbose {
            eprintln!("Connected to niri IPC socket");
        }
    }

    fn log_streaming(&self) {
        if self.json_verbose() {
            println!("{}", serde_json::json!({"event": "streaming"}));
        } else if self.verbose {
            eprintln!("Event stream started");
        }
    }

    fn log_target_loaded(&self, window: &Window) {
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

    fn log_target_changed(&self, window: &Window) {
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

    fn log_target_matched(&self, window: &Window, match_type: &str) {
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

impl Logger<Monitor> for GenericLogger {
    fn log_connected(&self) {
        if self.json_verbose() {
            println!("{}", serde_json::json!({"event": "connected"}));
        } else if self.verbose {
            eprintln!("Connected to niri IPC socket");
        }
    }

    fn log_streaming(&self) {
        if self.json_verbose() {
            println!("{}", serde_json::json!({"event": "streaming"}));
        } else if self.verbose {
            eprintln!("Event stream started");
        }
    }

    fn log_target_loaded(&self, monitor: &Monitor) {
        if self.json_verbose() {
            println!("{}", serde_json::json!({
                "event": "monitor-loaded",
                "id": monitor.id,
                "name": monitor.name,
                "description": monitor.description
            }));
        } else if self.verbose {
            eprintln!("Monitor loaded: id={}, name={:?}, description={:?}", monitor.id, monitor.name, monitor.description);
        }
    }

    fn log_target_changed(&self, monitor: &Monitor) {
        if self.json_verbose() {
            println!("{}", serde_json::json!({
                "event": "monitor-changed",
                "id": monitor.id,
                "name": monitor.name,
                "description": monitor.description
            }));
        } else if self.verbose {
            eprintln!(
                "Monitor changed: id={}, name={:?}, description={:?}",
                monitor.id,
                monitor.name,
                monitor.description
            );
        }
    }

    fn log_focus_change(&self, monitor_id: Option<u64>, monitor: Option<&Monitor>) {
        if self.json_verbose() {
            if let Some(id) = monitor_id {
                if let Some(m) = monitor {
                    println!("{}", serde_json::json!({
                        "event": "focus-change",
                        "id": id,
                        "name": m.name,
                        "description": m.description
                    }));
                } else {
                    println!("{}", serde_json::json!({
                        "event": "focus-change",
                        "id": id,
                        "name": null,
                        "description": null
                    }));
                }
            } else {
                println!("{}", serde_json::json!({
                    "event": "focus-change",
                    "id": null
                }));
            }
        } else if self.verbose {
            let id_str = monitor_id.map(|i| i.to_string()).unwrap_or_else(|| "None".to_string());
            if let Some(_id) = monitor_id {
                if let Some(m) = monitor {
                    eprintln!("Monitor focus changed: id={}, name={:?}, description={:?}", id_str, m.name, m.description);
                } else {
                    eprintln!("Monitor focus changed: {} (monitor info not available yet)", id_str);
                }
            } else {
                eprintln!("Monitor focus changed: {}", id_str);
            }
        }
    }

    fn log_target_matched(&self, monitor: &Monitor, match_type: &str) {
        if self.json {
            println!("{}", serde_json::json!({
                "event": "monitor-matched",
                "match_type": match_type,
                "id": monitor.id,
                "name": monitor.name,
                "description": monitor.description
            }));
        } else if self.verbose {
            eprintln!(
                "Monitor matched! match_type={}, id={}, name={:?}, description={:?}",
                match_type, monitor.id, monitor.name, monitor.description
            );
        }
    }
}
