use uring::uring::Ring;

fn main() {
    // `strace` should contain `io_uring_setup(...) = n` and `close(n)`.
    let _ = Ring::new(1);
}
