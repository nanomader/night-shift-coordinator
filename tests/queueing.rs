use night_shift_coordinator::slice::MAX_PARALLEL_SLICES;

#[test]
fn max_parallel_slices_is_three() {
    assert_eq!(MAX_PARALLEL_SLICES, 3);
}
