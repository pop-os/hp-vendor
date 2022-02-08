use nix::fcntl::{fcntl, FcntlArg};
use std::{fs, os::unix::io::AsRawFd};

/// Lock whole file. Returns `EACCESS` or `EAGAIN` if already locked.
pub fn setlk(file: &fs::File) -> nix::Result<()> {
    fcntl(
        file.as_raw_fd(),
        FcntlArg::F_SETLK(&libc::flock {
            l_type: libc::F_WRLCK as _,
            l_whence: libc::SEEK_SET as _,
            l_start: 0,
            l_len: 0,
            l_pid: 0,
        }),
    )?;
    Ok(())
}
