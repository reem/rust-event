use mio::{MioError, EventLoopError};
use std::error::FromError;

pub type EventResult<T> = Result<T, EventError>;

#[deriving(Copy, Clone, Show, PartialEq)]
pub enum EventError {
    MioError(MioError)
}

impl FromError<MioError> for EventError {
    fn from_error(err: MioError) -> EventError {
        EventError::MioError(err)
    }
}

impl<H> FromError<EventLoopError<H>> for EventError {
    fn from_error(err: EventLoopError<H>) -> EventError {
        EventError::MioError(err.error)
    }
}

