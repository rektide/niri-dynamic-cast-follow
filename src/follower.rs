use niri_ipc::{Event, Request, Response};
use niri_ipc::socket::Socket;
use serde_json;
use crate::matcher::Matcher;
use crate::logger::Logger;
use crate::target::FollowerState;

pub struct Follower<S: FollowerState> {
    state: S,
    matcher: Box<dyn Matcher<S::Target>>,
    logger: Box<dyn Logger<S::Target>>,
    json: bool,
}

impl<S: FollowerState> Follower<S> {
    pub fn new(
        state: S,
        matcher: Box<dyn Matcher<S::Target>>,
        logger: Box<dyn Logger<S::Target>>,
        json: bool,
    ) -> Self {
        Follower {
            state,
            matcher,
            logger,
            json,
        }
    }

    pub fn run<F, H>(
        mut self,
        mut socket: Socket,
        populate_cache: F,
        mut handle_event: H,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce(&mut Socket, &mut S, &dyn Logger<S::Target>) -> Result<(), Box<dyn std::error::Error>>,
        H: FnMut(Event, &mut S, &dyn Matcher<S::Target>, &dyn Logger<S::Target>, bool) -> Result<(), Box<dyn std::error::Error>>,
    {
        self.logger.log_connected();

        populate_cache(&mut socket, &mut self.state, self.logger.as_ref())?;

        let reply = socket.send(Request::EventStream)?;
        if !matches!(reply, Ok(Response::Handled)) {
            self.log_error("Failed to start event stream");
            std::process::exit(1);
        }

        self.logger.log_streaming();

        let mut read_event = socket.read_events();
        loop {
            match read_event() {
                Ok(event) => {
                    if let Err(e) = handle_event(
                        event,
                        &mut self.state,
                        self.matcher.as_ref(),
                        self.logger.as_ref(),
                        self.json,
                    ) {
                        self.log_event_error(e);
                    }
                }
                Err(e) => {
                    self.log_event_error(Box::new(e));
                    break;
                }
            }
        }

        Ok(())
    }

    fn log_error(&self, msg: &str) {
        if self.json {
            eprintln!("{}", serde_json::json!({"event": "error", "message": msg}));
        } else {
            eprintln!("{}", msg);
        }
    }

    fn log_event_error(&self, e: Box<dyn std::error::Error>) {
        if self.json {
            eprintln!("{}", serde_json::json!({"event": "error", "message": e.to_string()}));
        } else {
            eprintln!("Error handling event: {}", e);
        }
    }
}
