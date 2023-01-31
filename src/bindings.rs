use std::mem::{size_of, MaybeUninit};
use std::os::raw::*;
use std::ptr::null;

use nix::libc::{memset, size_t, syscall, MAP_POPULATE, MAP_SHARED, PROT_READ, PROT_WRITE};
use nix::sys::mman::{MapFlags, ProtFlags};

pub type __u8 = c_uchar;
pub type __u16 = c_ushort;
pub type __s32 = c_int;
pub type __u32 = c_uint;
pub type __u64 = c_ulonglong;

#[repr(C)]
enum SyscallNumber {
    IO_URING_SETUP = 425,
    IO_URING_ENTER = 426,
}

#[repr(C)]
pub enum Feature {
    IORING_FEAT_SINGLE_MMAP = (1 << 0),
}

pub enum Enter {
    IORING_ENTER_GETEVENTS = (1 << 0),
}

#[repr(C)]
pub enum Offset {
    IORING_OFF_SQ_RING = 0x0,
    IORING_OFF_CQ_RING = 0x8000000,
    IORING_OFF_SQES = 0x10000000,
}

#[repr(C)]
pub enum Opcode {
    IORING_OP_NOP = 0,
    IORING_OP_READ = 22,
}

#[repr(C)]
pub struct io_uring_cqe {
    pub user_data: __u64,
    pub res: __s32,
    pub flags: __u32,
}

#[repr(C)]
pub union union1 {
    pub off: __u64,
    pub addr2: __u64,
}

#[repr(C)]
pub union union2 {
    pub addr: __u64,
    pub splice_off_in: __u64,
}

#[repr(C)]
pub union union3 {
    pub rw_flags: c_int,
    pub fsync_flags: __u32,
    pub poll_events: __u16,
    pub poll32_events: __u32,
    pub sync_range_flags: __u32,
    pub msg_flags: __u32,
    pub timeout_flags: __u32,
    pub accept_flags: __u32,
    pub cancel_flags: __u32,
    pub open_flags: __u32,
    pub statx_flags: __u32,
    pub fadvise_advice: __u32,
    pub splice_flags: __u32,
    pub rename_flags: __u32,
    pub unlink_flags: __u32,
    pub hardlink_flags: __u32,
}

#[repr(C, packed)]
pub union union4 {
    pub buf_index: __u16,
    pub buf_group: __u16,
}

#[repr(C)]
pub union union5 {
    pub splice_fd_in: __s32,
    pub file_index: __u32,
}

#[repr(C)]
pub struct io_uring_sqe {
    pub opcode: __u8,
    pub flags: __u8,
    pub ioprio: __u16,
    pub fd: __s32,
    pub u1: union1,
    pub u2: union2,
    pub len: __u32,
    pub u3: union3,
    pub user_data: __u64,
    pub u4: union4,
    pub personality: __u16,
    pub u5: union5,
    pub __pad2: [__u64; 2],
}

impl Default for io_uring_sqe {
    fn default() -> Self {
        let mut params = MaybeUninit::<Self>::uninit();
        unsafe {
            memset(params.as_mut_ptr() as *mut c_void, 0, size_of::<Self>());
            params.assume_init()
        }
    }
}

#[repr(C)]
pub struct io_sqring_offsets {
    pub head: __u32,
    pub tail: __u32,
    pub ring_mask: __u32,
    pub ring_entries: __u32,
    pub flags: __u32,
    pub dropped: __u32,
    pub array: __u32,
    pub resv1: __u32,
    pub resv2: __u64,
}

#[repr(C)]
pub struct io_cqring_offsets {
    pub head: __u32,
    pub tail: __u32,
    pub ring_mask: __u32,
    pub ring_entries: __u32,
    pub overflow: __u32,
    pub cqes: __u32,
    pub flags: __u32,
    pub resv1: __u32,
    pub resv2: __u64,
}

#[repr(C)]
pub struct io_uring_params {
    pub sq_entries: __u32,
    pub cq_entries: __u32,
    pub flags: __u32,
    pub sq_thread_cpu: __u32,
    pub sq_thread_idle: __u32,
    pub features: __u32,
    pub wq_fd: __u32,
    pub resv: [__u32; 3],
    pub sq_off: io_sqring_offsets,
    pub cq_off: io_cqring_offsets,
}

impl Default for io_uring_params {
    fn default() -> Self {
        let mut params = MaybeUninit::<Self>::uninit();
        unsafe {
            memset(params.as_mut_ptr() as *mut c_void, 0, size_of::<Self>());
            params.assume_init()
        }
    }
}

pub unsafe fn io_uring_setup(entries: c_uint, params: *mut io_uring_params) -> c_long {
    syscall(SyscallNumber::IO_URING_SETUP as c_long, entries, params)
}

pub unsafe fn io_uring_enter(
    ring_fd: c_int,
    to_submit: c_uint,
    min_complete: c_uint,
    flags: c_uint,
) -> c_long {
    syscall(
        SyscallNumber::IO_URING_ENTER as c_long,
        ring_fd,
        to_submit,
        min_complete,
        flags,
    )
}

pub unsafe fn mmap(fd: c_int, len: size_t, offset: Offset) -> *mut c_void {
    nix::libc::mmap(
        null::<c_void>().cast_mut(),
        len,
        PROT_READ | PROT_WRITE,
        MAP_SHARED | MAP_POPULATE,
        fd,
        offset as c_long,
    )
}
