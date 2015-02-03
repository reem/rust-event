use mio::MioError;
use std::error::FromError;

pub type EventResult<T> = Result<T, EventError>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EventError {
    MioError(MioError)
}

impl FromError<MioError> for EventError {
    fn from_error(err: MioError) -> EventError {
        EventError::MioError(err)
    }
}

