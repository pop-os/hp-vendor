use nix::{
    fcntl::{self, OFlag},
    sys::stat::Mode,
    unistd,
};
use std::str;

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
                    let record = str::from_utf8(&buf[..len]).unwrap();
                    println!("RECORD: {}", record);
                }
            }
        }
    }
}
