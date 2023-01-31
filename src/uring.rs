use std::ffi::c_uint;
use std::mem::size_of;
use std::os::fd::RawFd;
use std::sync::atomic::{fence, Ordering};

use nix::libc::{c_int, c_void, close, size_t};

use crate::bindings::Feature::IORING_FEAT_SINGLE_MMAP;
use crate::bindings::*;

pub struct Ring {
    fd: RawFd,
    submission_ring: SubmissionRing,
    completion_ring: CompletionRing,
    pending: u32, // pending operations to submit
}

impl Ring {
    // TODO: change return type to Result<Ring>
    pub fn new(entries: u32) -> Ring {
        let mut p = io_uring_params::default();
        unsafe {
            let fd = io_uring_setup(entries, &mut p) as i32;
            let ptrs = Ring::mem_map(fd, &p);
            let submission_ring = SubmissionRing::new(ptrs.sq_ring, ptrs.sqe_array, p.sq_off);
            let completion_ring = CompletionRing::new(ptrs.cq_ring, p.cq_off);
            Ring {
                fd,
                submission_ring,
                completion_ring,
                pending: 0,
            }
        }
    }

    pub fn add(&mut self, ring_op: RingOp) {
        unsafe {
            let entry = self.submission_ring.next();
            *entry = ring_op.sqe;
            self.pending += 1;
        }
    }

    pub fn submit(&mut self) -> u32 {
        let res = unsafe {
            io_uring_enter(
                self.fd,
                self.pending,
                0,
                Enter::IORING_ENTER_GETEVENTS as c_uint,
            )
        };
        self.pending = 0;
        res as u32
    }

    pub fn wait(&mut self) -> u32 {
        let mut results = 0;
        unsafe {
            let mut head = *self.completion_ring.head;
            let mask = *self.completion_ring.mask;
            loop {
                fence(Ordering::SeqCst);
                if head == *self.completion_ring.tail {
                    break;
                }
                let _cqe = self.completion_ring.cqes.add((head & mask) as usize);
                head += 1;
                results += 1;
            }
            *self.completion_ring.head = head;
            fence(Ordering::SeqCst);
            results
        }
    }

    unsafe fn mem_map(fd: RawFd, p: &io_uring_params) -> MemoryPointers {
        let mut sq_ring_size = p.sq_off.array + p.sq_entries * (size_of::<u32>() as u32);
        let mut cq_ring_size = p.cq_off.cqes + p.cq_entries * (size_of::<io_uring_cqe>() as u32);
        let sqe_array_size = p.sq_entries + (size_of::<io_uring_sqe>() as u32);

        let is_single_map = p.features & (IORING_FEAT_SINGLE_MMAP as u32) != 0;
        if is_single_map {
            if cq_ring_size > sq_ring_size {
                sq_ring_size = cq_ring_size;
            }
            cq_ring_size = sq_ring_size;
        }

        let sq_ring_ptr = mmap(fd, sq_ring_size as size_t, Offset::IORING_OFF_SQ_RING);
        let cq_ring_ptr = match is_single_map {
            true => sq_ring_ptr,
            false => mmap(fd, cq_ring_size as size_t, Offset::IORING_OFF_CQ_RING),
        };
        let sqe_array_ptr = mmap(fd, sqe_array_size as size_t, Offset::IORING_OFF_SQES);

        MemoryPointers {
            sq_ring: sq_ring_ptr,
            cq_ring: cq_ring_ptr,
            sqe_array: sqe_array_ptr,
        }
    }
}

impl Drop for Ring {
    fn drop(&mut self) {
        unsafe {
            close(self.fd as c_int);
        }
    }
}

struct SubmissionRing {
    head: *mut u32,
    tail: *mut u32,
    mask: *mut u32,
    entries: *mut u32,
    flags: *mut u32,
    array: *mut u32,
    sqes: *mut io_uring_sqe,
}

impl SubmissionRing {
    unsafe fn next(&self) -> *mut io_uring_sqe {
        let mut tail = *self.tail;
        fence(Ordering::SeqCst);
        let index = tail & *self.mask;
        *self.array.add(index as usize) = index;
        tail += 1;
        if *self.tail != tail {
            *self.tail = tail;
            fence(Ordering::SeqCst);
        }
        self.sqes.add(index as usize)
    }
}

impl SubmissionRing {
    // `sq_ring_ptr` and `sqe_array_ptr` are address returned by mmap-ing the OFF_SQ_RING and OFF_SQES respectively.
    unsafe fn new(
        sq_ring_ptr: *mut c_void,
        sqe_array_ptr: *mut c_void,
        offsets: io_sqring_offsets,
    ) -> Self {
        SubmissionRing {
            head: sq_ring_ptr.add(offsets.head as usize) as *mut u32,
            tail: sq_ring_ptr.add(offsets.tail as usize) as *mut u32,
            mask: sq_ring_ptr.add(offsets.ring_mask as usize) as *mut u32,
            entries: sq_ring_ptr.add(offsets.ring_entries as usize) as *mut u32,
            flags: sq_ring_ptr.add(offsets.flags as usize) as *mut u32,
            array: sq_ring_ptr.add(offsets.array as usize) as *mut u32,
            sqes: sqe_array_ptr as *mut io_uring_sqe,
        }
    }
}

struct CompletionRing {
    head: *mut u32,
    tail: *mut u32,
    mask: *mut u32,
    entries: *mut u32,
    cqes: *mut io_uring_cqe,
}

impl CompletionRing {
    // `cq_ring_ptr` is address returned by mmap-ing the OFF_CQ_RING.
    unsafe fn new(cq_ring_ptr: *mut c_void, offsets: io_cqring_offsets) -> Self {
        CompletionRing {
            head: cq_ring_ptr.add(offsets.head as usize) as *mut u32,
            tail: cq_ring_ptr.add(offsets.tail as usize) as *mut u32,
            mask: cq_ring_ptr.add(offsets.ring_mask as usize) as *mut u32,
            entries: cq_ring_ptr.add(offsets.ring_entries as usize) as *mut u32,
            cqes: cq_ring_ptr.add(offsets.cqes as usize) as *mut io_uring_cqe,
        }
    }
}

struct MemoryPointers {
    sq_ring: *mut c_void,
    cq_ring: *mut c_void,
    sqe_array: *mut c_void,
}

pub struct RingOp {
    sqe: io_uring_sqe,
}

impl RingOp {
    pub fn builder() -> RingOpBuilder {
        RingOpBuilder::default()
    }

    pub fn read_builder() -> RingOpBuilder {
        RingOp::builder_with_op(Opcode::IORING_OP_READ)
    }

    fn builder_with_op(opcode: Opcode) -> RingOpBuilder {
        RingOpBuilder::default().opcode(opcode)
    }
}

#[derive(Default)]
pub struct RingOpBuilder {
    sqe: io_uring_sqe,
}

impl RingOpBuilder {
    pub fn fd(mut self, fd: RawFd) -> RingOpBuilder {
        self.sqe.fd = fd;
        self
    }

    pub fn flags(mut self, flags: u32) -> RingOpBuilder {
        self.sqe.flags = flags as __u8;
        self
    }

    pub fn opcode(mut self, opcode: Opcode) -> RingOpBuilder {
        self.sqe.opcode = opcode as __u8;
        self
    }

    pub fn addr(mut self, addr: usize) -> RingOpBuilder {
        self.sqe.u2.addr = addr as __u64;
        self
    }

    pub fn len(mut self, len: usize) -> RingOpBuilder {
        self.sqe.len = len as __u32;
        self
    }

    pub fn off(mut self, off: usize) -> RingOpBuilder {
        self.sqe.u1.off = off as __u64;
        self
    }

    pub fn user_data<T>(mut self, user_data: &T) -> RingOpBuilder {
        self.sqe.user_data = user_data as *const T as __u64;
        self
    }

    pub fn build(self) -> RingOp {
        RingOp { sqe: self.sqe }
    }
}
