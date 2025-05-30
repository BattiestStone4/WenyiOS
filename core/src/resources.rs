use linux_raw_sys::general::{
    RLIM_INFINITY, RLIM_NLIMITS, RLIMIT_AS, RLIMIT_CORE, RLIMIT_CPU, RLIMIT_DATA, RLIMIT_FSIZE,
    RLIMIT_LOCKS, RLIMIT_MEMLOCK, RLIMIT_MSGQUEUE, RLIMIT_NICE, RLIMIT_NOFILE, RLIMIT_NPROC,
    RLIMIT_RSS, RLIMIT_RTPRIO, RLIMIT_RTTIME, RLIMIT_SIGPENDING, RLIMIT_STACK,
};
use num_enum::TryFromPrimitive;

pub const RLIMIT_INFINITY: u64 = u64::MAX;
const _: () = assert!(RLIMIT_INFINITY == RLIM_INFINITY as i64 as u64);

#[repr(u32)]
#[derive(TryFromPrimitive, Clone, Copy, Debug)]
pub enum ResourceLimitType {
    CPU = RLIMIT_CPU,
    FSIZE = RLIMIT_FSIZE,
    DATA = RLIMIT_DATA,
    STACK = RLIMIT_STACK,
    CORE = RLIMIT_CORE,
    RSS = RLIMIT_RSS,
    NPROC = RLIMIT_NPROC,
    NOFILE = RLIMIT_NOFILE,
    MEMLOCK = RLIMIT_MEMLOCK,
    AS = RLIMIT_AS,
    LOCKS = RLIMIT_LOCKS,
    SIGPENDING = RLIMIT_SIGPENDING,
    MSGQUEUE = RLIMIT_MSGQUEUE,
    NICE = RLIMIT_NICE,
    RTPRIO = RLIMIT_RTPRIO,
    RTTIME = RLIMIT_RTTIME,
}

impl ResourceLimitType {
    pub fn to_usize(self: &Self) -> usize {
        *self as usize
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct ResourceLimit {
    pub soft: u64,
    pub hard: u64,
}

impl ResourceLimit {
    /// Create a new ResourceLimit with the same values as the given limit
    pub fn new(soft: u64, hard: u64) -> Self {
        Self { soft, hard }
    }

    /// Create a new ResourceLimit with infinite values
    pub fn new_infinite() -> Self {
        Self {
            soft: RLIMIT_INFINITY,
            hard: RLIMIT_INFINITY,
        }
    }
}

pub struct ResourceLimits([ResourceLimit; RLIM_NLIMITS as usize]);

impl ResourceLimits {
    pub fn new() -> Self {
        let mut limits = [ResourceLimit::new_infinite(); RLIM_NLIMITS as usize];
        limits[ResourceLimitType::STACK as usize] =
            ResourceLimit::new(axconfig::plat::USER_STACK_SIZE as u64, RLIMIT_INFINITY);
        limits[ResourceLimitType::CORE as usize] = ResourceLimit::new(0, RLIMIT_INFINITY);
        limits[ResourceLimitType::NPROC as usize] = ResourceLimit::new(10000, 10000);
        limits[ResourceLimitType::NOFILE as usize] = ResourceLimit::new(1024, 1024 * 1024); // 1024 files, 1M files max
        Self(limits)
    }

    pub fn get_soft(&self, resource: &ResourceLimitType) -> u64 {
        self.0[*resource as usize].soft
    }

    pub fn get(&self, resource: &ResourceLimitType) -> ResourceLimit {
        self.0[*resource as usize].clone()
    }

    pub fn set(&mut self, resource: &ResourceLimitType, limit: ResourceLimit) -> bool {
        if limit.soft > limit.hard {
            return false;
        }
        self.0[*resource as usize] = limit;
        true
    }
}
