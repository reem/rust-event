use mio::{MioError, TimerError};
use std::error::FromError;

pub type EventResult<T> = Result<T, EventError>;

#[derive(Debug)]
pub enum EventError {
    MioError(MioError),
    TimerError(TimerError)
}

impl FromError<MioError> for EventError {
    fn from_error(err: MioError) -> EventError {
        EventError::MioError(err)
    }
}

impl FromError<TimerError> for EventError {
    fn from_error(err: TimerError) -> EventError {
        EventError::TimerError(err)
    }
}

