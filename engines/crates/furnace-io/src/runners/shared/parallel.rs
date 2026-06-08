pub(in crate::runners) fn should_parallelize(
    group_count: usize,
    input_row_count: usize,
    worker_threads: usize,
) -> bool {
    worker_threads > 1
        && group_count >= worker_threads.saturating_mul(2).max(2)
        && input_row_count > 0
}
