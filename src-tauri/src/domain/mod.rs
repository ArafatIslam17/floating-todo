//! Pure domain logic: types, validation, reminder planning and time.

pub mod clock;
pub mod reminder;
pub mod task;
pub mod validate;

pub use clock::{Clock, SystemClock};
pub use task::*;
