use chrono::format::{DelayedFormat, StrftimeItems};

/// Gets the time in a pretty format. Ideal for logging.
///
/// # Returns
/// The formatted time, e.g. `02/05 11:23:15 PM`
#[inline]
pub fn get_pretty_time() -> DelayedFormat<StrftimeItems<'static>> {
    let time = chrono::offset::Local::now();
    time.format("%m/%d %I:%M:%S %p")
}

/// Returns the number of non-leap-milliseconds since January 1, 1970 UTC
///
/// This is essentially just an alias for `chrono::offset::Local.now().timestamp_millis`.
///
/// # Returns
/// The number of non-leap-milliseconds since January 1, 1970 UTC.
#[inline]
pub fn get_epoch_time() -> i64 {
    chrono::offset::Local::now().timestamp_millis()
}