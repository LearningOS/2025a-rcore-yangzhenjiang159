//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next},
    timer::get_time_us,
};
use crate::sync::UPSafeCell;
use lazy_static::*;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Trace request types
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum TraceRequest {
    Read = 0,
    Write = 1,
    Syscall = 2,
}

/// Global syscall counter
struct SyscallCounter {
    counts: [usize; 512], // Support up to 512 different syscall IDs
}

impl SyscallCounter {
    fn new() -> Self {
        Self {
            counts: [0; 512],
        }
    }
    
    fn increment(&mut self, syscall_id: usize) {
        if syscall_id < 512 {
            self.counts[syscall_id] += 1;
        }
    }
    
    fn get_count(&self, syscall_id: usize) -> usize {
        if syscall_id < 512 {
            self.counts[syscall_id]
        } else {
            0
        }
    }
}

lazy_static! {
    static ref SYSCALL_COUNTER: UPSafeCell<SyscallCounter> = unsafe {
        UPSafeCell::new(SyscallCounter::new())
    };
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// Trace syscall implementation
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    
    let request = match trace_request {
        0 => TraceRequest::Read,
        1 => TraceRequest::Write,
        2 => TraceRequest::Syscall,
        _ => return -1,
    };
    
    match request {
        TraceRequest::Read => {
            // Read from memory address
            let addr = id as *const u8;
            unsafe {
                // Check if the address is valid (basic bounds checking)
                if (addr as usize) < 0x1000 {
                    // Invalid address (below page boundary)
                    return -1;
                }
                // Try to read the value
                let value = core::ptr::read_volatile(addr);
                value as isize
            }
        }
        TraceRequest::Write => {
            // Write to memory address
            let addr = id as *mut u8;
            let value = data as u8;
            unsafe {
                // Check if the address is valid (basic bounds checking)
                if (addr as usize) < 0x1000 {
                    // Invalid address (below page boundary)
                    return -1;
                }
                // Try to write the value
                core::ptr::write_volatile(addr, value);
                0 // Success
            }
        }
        TraceRequest::Syscall => {
            // Get syscall count
            let counter = SYSCALL_COUNTER.exclusive_access();
            counter.get_count(id) as isize
        }
    }
}
