use nix::{
    fcntl::{self, OFlag},
    sys::stat::Mode,
    unistd,
};
use std::str;

// https://www.kernel.org/doc/Documentation/ABI/testing/dev-kmsg
fn parse_kmsg(buf: &[u8]) -> Option<()> {
    let record = str::from_utf8(buf).ok()?;
    let mut lines = record.lines();

    let (_props, message) = lines.next()?.split_once(';')?;

    let mut subsystem = None;
    let mut device = None;
    for i in lines {
        if let Some(value) = i.strip_prefix(" SUBSYSTEM=") {
            subsystem = Some(value);
        } else if let Some(value) = i.strip_prefix(" DEVICE=") {
            device = Some(value);
        }
    }
    println!("RECORD({:?}, {:?}): {}", subsystem, device, message);
    Some(()) // XXX
}

fn main() {
    let mut poll = mio::Poll::new().unwrap();
    let mut events = mio::Events::with_capacity(1024);

    let mut socket = udev::MonitorBuilder::new()
        .unwrap()
        .match_subsystem_devtype("usb", "usb_device")
        .unwrap()
        .listen()
        .unwrap();
    poll.registry()
        .register(
            &mut socket,
            mio::Token(0),
            mio::Interest::READABLE | mio::Interest::WRITABLE,
        )
        .unwrap();

    let fd = fcntl::open(
        "/dev/kmsg",
        OFlag::O_RDONLY | OFlag::O_NONBLOCK,
        Mode::empty(),
    )
    .unwrap();
    unistd::lseek(fd, 0, unistd::Whence::SeekEnd).unwrap();
    let mut kmsg_source = mio::unix::SourceFd(&fd);
    poll.registry()
        .register(&mut kmsg_source, mio::Token(1), mio::Interest::READABLE)
        .unwrap();

    loop {
        poll.poll(&mut events, None).unwrap();

        for event in &events {
            if event.token() == mio::Token(0) && event.is_writable() {
                socket.clone().for_each(|x| {
                    println!("{:?}: {:?}", x.event_type(), x.syspath());
                });
            } else if event.token() == mio::Token(1) {
                let mut buf = [0; 1024];
                while let Ok(len) = unistd::read(fd, &mut buf) {
                    parse_kmsg(&buf[..len]);
                }
            }
        }
    }
}
