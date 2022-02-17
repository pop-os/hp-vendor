use nix::{
    errno::Errno,
    fcntl::{fcntl, FcntlArg},
};
use std::{fs, os::unix::io::AsRawFd};

pub mod dmi;
pub mod drm;
pub mod nvme;

/// Set unique advisory lock on whole file Returns `EACCESS` or `EAGAIN` if
/// already locked.
fn setlk(file: &fs::File) -> nix::Result<()> {
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

// Panics if file can't be opened or lock is held
pub fn lock_file_or_panic(path: &str) -> Lock {
    let file = match fs::File::create(path) {
        Ok(file) => file,
        Err(err) => panic!("Failed to open `{}`: {}", path, err),
    };
    if let Err(err) = setlk(&file) {
        if err == Errno::EACCES || err == Errno::EAGAIN {
            panic!("Lock already held on `{}`", path);
        } else {
            panic!("Error locking `{}`: {}", path, err);
        }
    }
    Lock(file)
}

pub struct Lock(fs::File);
