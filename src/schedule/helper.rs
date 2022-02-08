use super::scheduler::Time;

/// Calculates the time after a given offset.
///
/// # Parameters
/// - `time`: The time.
/// - `offset`: The offset.
///
/// # Returns
/// The new `Time` with the specified offset accounted for.
pub fn calculate_time_with_offset(time: Time, offset: i16) -> Time {
    let (mut hr, mut min) = time;
    min += offset;
    if min >= 60 {
        while min >= 60 {
            hr += 1;
            min -= 60;
        }
    } else if min < 0 {
        while min < 0 {
            hr -= 1;
            min += 60;
        }
    }

    (hr, min)
}

/// Checks if there is a time conflict between two time ranges.
///
/// # Parameters
/// - `a_from`: The start time of the first event.
/// - `a_to`: The end time of the first event.
/// - `b_from`: The start time of the second event.
/// - `b_to`: The end time of the second event.
///
/// # Returns
/// `true` if the time conflicts and `false` otherwise.
pub fn time_conflicts(a_from: Time, a_to: Time, b_from: Time, b_to: Time) -> bool {
    _time_conflicts(a_from, a_to, b_from, b_to) || _time_conflicts(b_from, b_to, a_from, a_to)
}

fn _time_conflicts(a_from: Time, a_to: Time, b_from: Time, b_to: Time) -> bool {
    let a_start = a_from.0 * 100 + a_from.1;
    let a_end = a_to.0 * 100 + a_to.1;
    let b_start = b_from.0 * 100 + b_from.1;
    let b_end = b_to.0 * 100 + b_to.1;

    // Case 1: 1100 - 1200 & 1030 - 1130
    if b_start <= a_start && a_start <= b_end {
        return true;
    }

    // Case 2: 1100 - 1200 & 1130 - 1230
    if b_start <= a_end && a_end <= b_end {
        return true;
    }

    // Case 3: 1100 - 12:00 & 1120 - 1140
    if a_start <= b_start && b_end <= a_end {
        return true;
    }

    false
}

#[cfg(test)]
mod offset_tests {
    use super::*;

    const BASE_TIME: (i16, i16) = (5, 10);

    #[test]
    fn basic_offset() {
        assert_eq!((5, 15), calculate_time_with_offset(BASE_TIME, 5));
    }

    #[test]
    fn basic_offset_2() {
        assert_eq!((5, 59), calculate_time_with_offset(BASE_TIME, 49));
    }

    #[test]
    fn offset_over_pos_1() {
        assert_eq!((6, 30), calculate_time_with_offset(BASE_TIME, 80));
    }

    #[test]
    fn offset_over_pos_2() {
        assert_eq!((7, 15), calculate_time_with_offset(BASE_TIME, 125));
    }

    #[test]
    fn offset_over_neg_1() {
        assert_eq!((4, 30), calculate_time_with_offset(BASE_TIME, -40));
    }

    #[test]
    fn offset_over_neg_2() {
        assert_eq!((3, 5), calculate_time_with_offset(BASE_TIME, -125));
    }
}

#[cfg(test)]
mod conflict_tests {
    use super::*;

    #[test]
    fn general_no_conflict() {
        assert!(!time_conflicts((10, 0), (10, 50), (11, 0), (11, 50)));
    }

    #[test]
    fn general_no_conflict_rev() {
        assert!(!time_conflicts((11, 0), (11, 50), (10, 0), (10, 50)));
    }

    #[test]
    fn close_no_conflict() {
        assert!(!time_conflicts((15, 20), (15, 30), (15, 31), (15, 33)));
    }

    #[test]
    fn close_no_conflict_rev() {
        assert!(!time_conflicts((15, 31), (15, 33), (15, 20), (15, 30)));
    }

    #[test]
    fn right_conflict() {
        assert!(time_conflicts((10, 0), (10, 50), (10, 15), (11, 50)));
    }

    #[test]
    fn right_conflict_rev() {
        assert!(time_conflicts((10, 15), (11, 50), (10, 0), (10, 50)));
    }

    #[test]
    fn close_right_conflict() {
        assert!(time_conflicts((19, 0), (19, 30), (19, 30), (19, 40)));
    }

    #[test]
    fn close_right_conflict_rev() {
        assert!(time_conflicts((19, 30), (19, 40), (19, 0), (19, 30)));
    }

    #[test]
    fn left_conflict() {
        assert!(time_conflicts((10, 15), (11, 0), (10, 0), (10, 50)));
    }

    #[test]
    fn left_conflict_rev() {
        assert!(time_conflicts((10, 0), (10, 50), (10, 15), (11, 0)));
    }

    #[test]
    fn close_left_conflict() {
        assert!(time_conflicts((9, 15), (10, 0), (10, 0), (10, 50)));
    }

    #[test]
    fn close_left_conflict_rev() {
        assert!(time_conflicts((10, 0), (10, 50), (9, 15), (10, 0)));
    }

    #[test]
    fn overlap_conflict() {
        assert!(time_conflicts((10, 0), (10, 50), (10, 20), (10, 40)));
    }

    #[test]
    fn overlap_conflict_rev() {
        assert!(time_conflicts((10, 20), (10, 40), (10, 0), (10, 50)));
    }

    #[test]
    fn full_overlap_conflict() {
        assert!(time_conflicts((10, 0), (10, 50), (10, 0), (10, 50)));
    }
}
