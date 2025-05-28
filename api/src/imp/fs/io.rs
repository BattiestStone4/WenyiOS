use core::ffi::{c_int, c_long};

use alloc::vec;
use axerrno::{LinuxError, LinuxResult};
use axio::SeekFrom;
use linux_raw_sys::general::{__kernel_off_t, iovec};

use crate::{
    file::{File, FileLike, get_file_like},
    ptr::{UserConstPtr, UserPtr, nullable},
};

/// Read data from the file indicated by `fd`.
///
/// Return the read size if success.
pub fn sys_read(fd: i32, buf: UserPtr<u8>, len: usize) -> LinuxResult<isize> {
    let buf = buf.get_as_mut_slice(len)?;
    debug!(
        "sys_read <= fd: {}, buf: {:p}, len: {}",
        fd,
        buf.as_ptr(),
        buf.len()
    );
    Ok(get_file_like(fd)?.read(buf)? as _)
}

pub fn sys_readv(fd: i32, iov: UserPtr<iovec>, iocnt: usize) -> LinuxResult<isize> {
    if !(0..=1024).contains(&iocnt) {
        return Err(LinuxError::EINVAL);
    }

    let iovs = iov.get_as_mut_slice(iocnt)?;
    let mut ret = 0;
    for iov in iovs {
        if iov.iov_len == 0 {
            continue;
        }
        let buf = UserPtr::<u8>::from(iov.iov_base as usize);
        let buf = buf.get_as_mut_slice(iov.iov_len as _)?;
        debug!(
            "sys_readv <= fd: {}, buf: {:p}, len: {}",
            fd,
            buf.as_ptr(),
            buf.len()
        );

        let read = get_file_like(fd)?.read(buf)?;
        ret += read as isize;

        if read < buf.len() {
            break;
        }
    }

    Ok(ret)
}

/// Write data to the file indicated by `fd`.
///
/// Return the written size if success.
pub fn sys_write(fd: i32, buf: UserConstPtr<u8>, len: usize) -> LinuxResult<isize> {
    let buf = buf.get_as_slice(len)?;
    debug!(
        "sys_write <= fd: {}, buf: {:p}, len: {}",
        fd,
        buf.as_ptr(),
        buf.len()
    );
    Ok(get_file_like(fd)?.write(buf)? as _)
}

pub fn sys_writev(fd: i32, iov: UserConstPtr<iovec>, iocnt: usize) -> LinuxResult<isize> {
    if !(0..=1024).contains(&iocnt) {
        return Err(LinuxError::EINVAL);
    }

    let iovs = iov.get_as_slice(iocnt)?;
    let mut ret = 0;
    for iov in iovs {
        if iov.iov_len == 0 {
            continue;
        }
        let buf = UserConstPtr::<u8>::from(iov.iov_base as usize);
        let buf = buf.get_as_slice(iov.iov_len as _)?;
        debug!(
            "sys_writev <= fd: {}, buf: {:p}, len: {}",
            fd,
            buf.as_ptr(),
            buf.len()
        );

        let written = get_file_like(fd)?.write(buf)?;
        ret += written as isize;

        if written < buf.len() {
            break;
        }
    }

    Ok(ret)
}

pub fn sys_lseek(fd: c_int, offset: __kernel_off_t, whence: c_int) -> LinuxResult<isize> {
    debug!("sys_lseek <= {} {} {}", fd, offset, whence);
    let pos = match whence {
        0 => SeekFrom::Start(offset as _),
        1 => SeekFrom::Current(offset as _),
        2 => SeekFrom::End(offset as _),
        _ => return Err(LinuxError::EINVAL),
    };
    let off = File::from_fd(fd)?.inner().seek(pos)?;
    Ok(off as _)
}

fn do_sendfile<F, D>(mut read: F, dest: &D) -> LinuxResult<usize>
where
    F: FnMut(&mut [u8]) -> LinuxResult<usize>,
    D: FileLike + ?Sized,
{
    let mut buf = vec![0; 0x4000];
    let mut total_written = 0;
    loop {
        let bytes_read = read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }

        let bytes_written = dest.write(&buf[..bytes_read])?;
        if bytes_written < bytes_read {
            break;
        }
        total_written += bytes_written;
    }

    Ok(total_written)
}

pub fn sys_sendfile(
    out_fd: c_int,
    in_fd: c_int,
    offset: UserPtr<u64>,
    len: usize,
) -> LinuxResult<isize> {
    debug!(
        "sys_sendfile <= out_fd: {}, in_fd: {}, offset: {}, len: {}",
        out_fd,
        in_fd,
        !offset.is_null(),
        len
    );

    let src = get_file_like(in_fd)?;
    let dest = get_file_like(out_fd)?;
    let offset = nullable!(offset.get_as_mut())?;

    if let Some(offset) = offset {
        let src = src
            .into_any()
            .downcast::<File>()
            .map_err(|_| LinuxError::ESPIPE)?;

        do_sendfile(
            |buf| {
                let bytes_read = src.inner().read_at(*offset, buf)?;
                *offset += bytes_read as u64;
                Ok(bytes_read)
            },
            dest.as_ref(),
        )
    } else {
        do_sendfile(|buf| src.read(buf), dest.as_ref())
    }
    .map(|n| n as _)
}

pub fn sys_ftruncate(fd: c_int, length: c_long) -> LinuxResult<isize> {
    let file_like = get_file_like(fd)?.into_any();
    let api_file = file_like.downcast_ref::<File>().ok_or(LinuxError::EINVAL)?;
    api_file
        .inner()
        .truncate(length as u64)
        .map_err(|_| axerrno::LinuxError::EIO)?;
    Ok(0)
}
