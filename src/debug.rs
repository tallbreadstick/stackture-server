use std::error::Error;
use chrono::Local;
use color_print::cprintln;

pub enum LogType {
    SETUP,
    HTTP,
    WEBSOCKET
}

pub fn log(log_type: LogType, message: &str) {
    match log_type {
        LogType::SETUP => cprintln!("{}<yellow>[SETUP]</yellow><green>[LOG]</green>: {}", now(), message),
        LogType::HTTP => cprintln!("{}<cyan>[REQUESTED]</cyan><green>[LOG]</green>: {}", now(), message),
        LogType::WEBSOCKET => cprintln!("{}<magenta>[CONNECTED]</magenta><green>[LOG]</green>: {}", now(), message)
    }
}

pub fn errlog(log_type: LogType, error: &impl Error) {
    match log_type {
        LogType::SETUP => cprintln!("{}<yellow>[SETUP]</yellow><red>[ERROR]</red>: {}", now(), error),
        LogType::HTTP => cprintln!("{}<cyan>[REQUESTED]</cyan><red>[ERROR]</red>: {}", now(), error),
        LogType::WEBSOCKET => cprintln!("{}<magenta>[CONNECTED]</magenta><red>[ERROR]</red>: {}", now(), error)
    }
}

fn now() -> String {
    Local::now()
        .format("[%d/%m/%y-%I:%M%p]")
        .to_string()
}