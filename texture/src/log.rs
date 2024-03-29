use std::{fmt::Display, panic::Location};

use log::{Level, Record};

fn log_error<E: Display>(err: E, location: &Location) {
    log::logger().log(
        &Record::builder()
            .args(format_args!("Unexpected error {err} at {location}"))
            .file(Some(location.file()))
            .line(Some(location.line()))
            .level(Level::Error)
            .build(),
    );
}

pub trait OkLog<T> {
    fn ok_log(self) -> Option<T>;
}

impl<T, E> OkLog<T> for std::result::Result<T, E>
where
    E: Display,
{
    #[track_caller]
    fn ok_log(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(err) => {
                let location = Location::caller();
                log_error(err, location);
                None
            }
        }
    }
}
