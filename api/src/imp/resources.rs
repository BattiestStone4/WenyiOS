use axerrno::{LinuxError, LinuxResult};
use axprocess::Pid;
use axtask::{TaskExtRef, current};
use core::ffi::c_int;
use linux_raw_sys::general::{RLIM_NLIMITS, rlimit64};
use starry_core::resources::{ResourceLimit, ResourceLimitType};
use starry_core::task::{ProcessData, get_process, get_thread};

use crate::ptr::{UserConstPtr, UserPtr, nullable};

pub fn sys_prlimit64(
    pid: c_int,
    resource: c_int,
    new_limit: UserConstPtr<ResourceLimit>,
    old_limit: UserPtr<ResourceLimit>,
) -> LinuxResult<isize> {
    let resource = ResourceLimitType::try_from(resource as u32).map_err(|_| LinuxError::EINVAL)?;
    if !old_limit.is_null() {
        let old_value = do_getrlimit(&resource, pid as _)?;
        old_limit.get_as_mut().unwrap().hard = old_value.hard;
        old_limit.get_as_mut().unwrap().soft = old_value.soft;
    }
    if !new_limit.is_null() {
        let new_value = new_limit.get_as_ref()?;
        do_setrlimit(&resource, new_value, pid as _)?;
    }
    Ok(0)
}

pub fn sys_setrlimit(
    resource: c_int,
    resource_limit: UserConstPtr<ResourceLimit>,
) -> LinuxResult<isize> {
    let resource = ResourceLimitType::try_from(resource as u32).map_err(|_| LinuxError::EINVAL)?;
    do_setrlimit(&resource, resource_limit.get_as_ref()?, 0)
}

pub fn sys_getrlimit(
    resource: c_int,
    resource_limit: UserPtr<ResourceLimit>,
) -> LinuxResult<isize> {
    let resource = ResourceLimitType::try_from(resource as u32).map_err(|_| LinuxError::EINVAL)?;
    let old_value = do_getrlimit(&resource, 0)?;
    {
        resource_limit.get_as_mut()?.hard = old_value.hard;
        resource_limit.get_as_mut()?.soft = old_value.soft;
    }
    Ok(0)
}

fn do_setrlimit(
    resource: &ResourceLimitType,
    limit: &ResourceLimit,
    pid: Pid,
) -> LinuxResult<isize> {
    let proc = if pid == 0 {
        current().task_ext().thread.process().clone()
    } else {
        get_process(pid)?
    };

    let proc_data: &ProcessData = proc.data().unwrap();
    let mut limits = proc_data.resource_limits.lock();
    let old_limit = limits.get(resource);
    if limit.hard > old_limit.hard {
        return Err(LinuxError::EPERM);
    }
    if !limits.set(resource, limit.clone()) {
        return Err(LinuxError::EINVAL); // soft > hard
    }
    Ok(0)
}
fn do_getrlimit(resource: &ResourceLimitType, pid: Pid) -> LinuxResult<ResourceLimit> {
    let proc = if pid == 0 {
        current().task_ext().thread.process().clone()
    } else {
        get_process(pid)?
    };

    let proc_data: &ProcessData = proc.data().unwrap();
    let mut limits = proc_data.resource_limits.lock();
    Ok(limits.get(resource))
}
