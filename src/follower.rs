use niri_ipc::{Event, Request, Response};
use niri_ipc::socket::Socket;
use serde_json;
use crate::target::Target;
use crate::matcher::Matcher;
use crate::logger::Logger;

pub struct Follower<T>
where
    T: Target,
{
    state: T,
    matcher: Box<dyn Matcher<T>>,
    logger: Box<dyn Logger<T>>,
    json: bool,
}

impl<T> Follower<T>
where
    T: Target,
{
    pub fn new(
        state: T,
        matcher: Box<dyn Matcher<T>>,
        logger: Box<dyn Logger<T>>,
        json: bool,
    ) -> Self {
        Follower {
            state,
            matcher,
            logger,
            json,
        }
    }

    pub fn run<F>(
        mut self,
        socket: &mut Socket,
        populate_cache: F,
        handle_event: fn(Event, &mut T, &dyn Matcher<T>, &dyn Logger<T>, bool) -> Result<(), Box<dyn std::error::Error>>,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce(&mut Socket, &mut T, &dyn Logger<T>) -> Result<(), Box<dyn std::error::Error>>,
    {
        self.logger.log_connected();

        populate_cache(socket, &mut self.state, self.logger.as_ref())?;

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
                    self.log_event_error(e);
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
