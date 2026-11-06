use super::*;

#[test]
fn a_domain_label() {
    announce_domain("Motion control locking");
}

#[test]
fn control_second_acquire_fails_until_released() {
    let _serial = busy_test_lock();
    assert!(!is_busy());
    let first = BusyGuard::acquire().expect("first acquire should succeed");
    assert!(is_busy());
    assert!(
        BusyGuard::acquire().is_none(),
        "a second acquire must fail while a motion is in progress"
    );
    drop(first);
    assert!(!is_busy());
    let second = BusyGuard::acquire().expect("acquire should succeed again after release");
    drop(second);
    assert!(!is_busy());
}

#[test]
fn control_guard_releases_the_flag_even_after_a_panic() {
    let _serial = busy_test_lock();
    assert!(!is_busy());
    let result = std::panic::catch_unwind(|| {
        let _guard = BusyGuard::acquire().unwrap();
        assert!(is_busy());
        panic!("simulated failure while a move is in progress");
    });
    assert!(result.is_err());
    assert!(
        !is_busy(),
        "the guard must release the flag during unwind, not just on a normal return"
    );
}

