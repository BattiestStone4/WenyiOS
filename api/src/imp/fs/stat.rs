use core::ffi::{c_char, c_int};

use axerrno::{AxError, LinuxError, LinuxResult};
use axfs::fops::OpenOptions;
use bitflags::bitflags;
use linux_raw_sys::general::{AT_EMPTY_PATH, R_OK, W_OK, X_OK, stat, statx};

use crate::path::resolve_path_with_parent;
use crate::{
    file::{Directory, File, FileLike, Kstat, get_file_like},
    path::handle_file_path,
    ptr::{UserConstPtr, UserPtr, nullable},
};

fn stat_at_path(path: &str) -> LinuxResult<Kstat> {
    let opts = OpenOptions::new().set_read(true);
    match axfs::fops::File::open(path, &opts) {
        Ok(file) => File::new(file, path.into()).stat(),
        Err(AxError::IsADirectory) => {
            let dir = axfs::fops::Directory::open_dir(path, &opts)?;
            Directory::new(dir, path.into()).stat()
        }
        Err(e) => Err(e.into()),
    }
}

/// Get the file metadata by `path` and write into `statbuf`.
///
/// Return 0 if success.
pub fn sys_stat(path: UserConstPtr<c_char>, statbuf: UserPtr<stat>) -> LinuxResult<isize> {
    let path = path.get_as_str()?;
    debug!("sys_stat <= path: {}", path);

    *statbuf.get_as_mut()? = stat_at_path(path)?.into();

    Ok(0)
}

/// Get file metadata by `fd` and write into `statbuf`.
///
/// Return 0 if success.
pub fn sys_fstat(fd: i32, statbuf: UserPtr<stat>) -> LinuxResult<isize> {
    debug!("sys_fstat <= fd: {}", fd);
    *statbuf.get_as_mut()? = get_file_like(fd)?.stat()?.into();
    Ok(0)
}

/// Get the metadata of the symbolic link and write into `buf`.
///
/// Return 0 if success.
pub fn sys_lstat(path: UserConstPtr<c_char>, statbuf: UserPtr<stat>) -> LinuxResult<isize> {
    // TODO: symlink
    sys_stat(path, statbuf)
}

pub fn sys_fstatat(
    dirfd: c_int,
    path: UserConstPtr<c_char>,
    statbuf: UserPtr<stat>,
    flags: u32,
) -> LinuxResult<isize> {
    let path = nullable!(path.get_as_str())?;
    debug!(
        "sys_fstatat <= dirfd: {}, path: {:?}, flags: {}",
        dirfd, path, flags
    );

    *statbuf.get_as_mut()? = if path.is_none_or(|s| s.is_empty()) {
        if (flags & AT_EMPTY_PATH) == 0 {
            return Err(LinuxError::ENOENT);
        }
        let f = get_file_like(dirfd)?;
        f.stat()?.into()
    } else {
        let path = handle_file_path(dirfd, path.unwrap_or_default())?;
        stat_at_path(path.as_str())?.into()
    };

    Ok(0)
}

pub fn sys_statx(
    dirfd: c_int,
    path: UserConstPtr<c_char>,
    flags: u32,
    _mask: u32,
    statxbuf: UserPtr<statx>,
) -> LinuxResult<isize> {
    // `statx()` uses pathname, dirfd, and flags to identify the target
    // file in one of the following ways:

    // An absolute pathname(situation 1)
    //        If pathname begins with a slash, then it is an absolute
    //        pathname that identifies the target file.  In this case,
    //        dirfd is ignored.

    // A relative pathname(situation 2)
    //        If pathname is a string that begins with a character other
    //        than a slash and dirfd is AT_FDCWD, then pathname is a
    //        relative pathname that is interpreted relative to the
    //        process's current working directory.

    // A directory-relative pathname(situation 3)
    //        If pathname is a string that begins with a character other
    //        than a slash and dirfd is a file descriptor that refers to
    //        a directory, then pathname is a relative pathname that is
    //        interpreted relative to the directory referred to by dirfd.
    //        (See openat(2) for an explanation of why this is useful.)

    // By file descriptor(situation 4)
    //        If pathname is an empty string (or NULL since Linux 6.11)
    //        and the AT_EMPTY_PATH flag is specified in flags (see
    //        below), then the target file is the one referred to by the
    //        file descriptor dirfd.

    let path = nullable!(path.get_as_str())?;
    debug!(
        "sys_statx <= dirfd: {}, path: {:?}, flags: {}",
        dirfd, path, flags
    );

    *statxbuf.get_as_mut()? = if path.is_none_or(|s| s.is_empty()) {
        if (flags & AT_EMPTY_PATH) == 0 {
            return Err(LinuxError::ENOENT);
        }
        let f = get_file_like(dirfd)?;
        f.stat()?.into()
    } else {
        let path = handle_file_path(dirfd, path.unwrap_or_default())?;
        stat_at_path(path.as_str())?.into()
    };

    Ok(0)
}

/// statfs - get filesystem statistics
/// Standard C library (libc, -lc)
/// <https://man7.org/linux/man-pages/man2/statfs.2.html>
#[repr(C)]
#[derive(Debug, Default)]
pub struct StatFs {
    /// Type of filesystem (see below)
    pub f_type: FsWord,
    /// Optimal transfer block size
    pub f_bsize: FsWord,
    /// Total data blocks in filesystem
    pub f_blocks: FsBlkCnt,
    /// Free blocks in filesystem
    pub f_bfree: FsBlkCnt,
    /// Free blocks available to unprivileged user
    pub f_bavail: FsBlkCnt,
    /// Total inodes in filesystem
    pub f_files: FsFilCnt,
    /// Free inodes in filesystem
    pub f_ffree: FsFilCnt,
    /// Filesystem ID
    pub f_fsid: FsId,
    /// Maximum length of filenames
    pub f_namelen: FsWord,
    /// Fragment size (since Linux 2.6)
    pub f_frsize: FsWord,
    /// Mount flags of filesystem (since Linux 2.6.36)
    pub f_flags: FsWord,
    /// Padding bytes reserved for future use
    pub f_spare: [FsWord; 5],
}

/// Type of miscellaneous file system fields. (typedef long __fsword_t)
pub type FsWord = isize;

/// Type to count file system blocks. (typedef unsigned long __fsblkcnt_t)
pub type FsBlkCnt = usize;

/// Type to count file system nodes. (typedef unsigned long __fsfilcnt_t)
pub type FsFilCnt = usize;

/// Type of file system IDs.
#[repr(C)]
#[derive(Debug, Default)]
pub struct FsId {
    /// raw value of the ID
    pub val: [i32; 2],
}

pub struct FsType;

impl FsType {
    const EXT4_SUPER_MAGIC: u32 = 0xEF53;
}

// TODO: [dummy] return dummy values
pub fn sys_statfs(path: UserConstPtr<c_char>, buf: UserPtr<StatFs>) -> LinuxResult<isize> {
    let path = path.get_as_str()?;
    let _ = handle_file_path(-1, path)?;

    // dummy data
    let stat_fs = StatFs {
        f_type: FsType::EXT4_SUPER_MAGIC as _,
        f_bsize: 4096,
        f_namelen: 255,
        f_frsize: 4096,
        f_blocks: 100000,
        f_bfree: 50000,
        f_bavail: 40000,
        f_files: 1000,
        f_ffree: 500,
        ..Default::default()
    };
    
    let buf = buf.get_as_mut()?;
    *buf = stat_fs;

    Ok(0)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_access(path: UserConstPtr<c_char>, mode: u32) -> LinuxResult<isize> {
    use linux_raw_sys::general::AT_FDCWD;

    sys_faccessat2(AT_FDCWD, path, mode, 0)
}

pub fn sys_faccessat2(
    dirfd: c_int,
    path: UserConstPtr<c_char>,
    mode: u32,
    flags: u32,
) -> LinuxResult<isize> {
    let path = nullable!(path.get_as_str())?;
    
    if mode == 0 {
        return Ok(0);
    };
    
    let mode = AccessFlags::from_bits(mode).ok_or(LinuxError::EINVAL)?;
    let path = resolve_path_with_parent(dirfd, path.unwrap())?;
    let mut options = OpenOptions::new();
    options.read(true);
    let permissions = if let Ok(file) = axfs::fops::File::open(&path, &options) {
        file.get_attr()?.perm()
    } else if let Ok(dir) = axfs::fops::Directory::open_dir(&path, &options) {
        dir.get_attr()?.perm()
    } else {
        return Err(LinuxError::ENOENT);
    };
    
    let mut access = true;
    if mode.contains(AccessFlags::R_OK) {
        access |= permissions.owner_readable();
    }
    if mode.contains(AccessFlags::W_OK) {
        access |= permissions.owner_writable();
    }
    if mode.contains(AccessFlags::X_OK) {
        access |= permissions.owner_executable();
    }

    if access {
        Ok(0)
    } else {
        Err(LinuxError::EACCES)
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct AccessFlags: u32 {
        const R_OK = R_OK;
        const W_OK = W_OK;
        const X_OK = X_OK;
    }
}
