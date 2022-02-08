use nix::{
    errno::Errno,
    fcntl::{fcntl, FcntlArg},
};
use std::{fs, os::unix::io::AsRawFd};

/// Set unique advisory lock on whole file Returns `EACCESS` or `EAGAIN` if
/// already locked.
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
    )
    .map(|_| ())
}

pub fn setlk_wait(file: &fs::File) -> nix::Result<()> {
    loop {
        let res = fcntl(
            file.as_raw_fd(),
            FcntlArg::F_SETLKW(&libc::flock {
                l_type: libc::F_WRLCK as _,
                l_whence: libc::SEEK_SET as _,
                l_start: 0,
                l_len: 0,
                l_pid: 0,
            }),
        );
        if res != Err(Errno::EINTR) {
            return res.map(|_| ());
        }
    }
}

pub fn unlck(file: &fs::File) -> nix::Result<()> {
    fcntl(
        file.as_raw_fd(),
        FcntlArg::F_SETLK(&libc::flock {
            l_type: libc::F_UNLCK as _,
            l_whence: libc::SEEK_SET as _,
            l_start: 0,
            l_len: 0,
            l_pid: 0,
        }),
    )?;
    Ok(())
}
