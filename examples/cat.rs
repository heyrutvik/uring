use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::time::Duration;
use std::{env, thread};

use uring::uring::{Ring, RingOp};

fn main() {
    let args: Vec<String> = env::args().collect();
    let input = &args[1];
    let file = File::open(input).unwrap();
    let fd = file.as_raw_fd();
    let size = file.metadata().unwrap().size();
    let mut buffer = vec![0; size as usize];

    let corr_id = 1;
    let read_op = RingOp::read_builder()
        .fd(fd)
        .addr(buffer.as_mut_ptr() as usize)
        .len(size as usize)
        .off(0)
        .flags(0)
        .user_data(&corr_id)
        .build();

    let mut ring = Ring::new(1);
    ring.add(read_op);
    ring.submit();

    while ring.wait() != 1 {}

    print!("{}", String::from_utf8(buffer).unwrap());
}
