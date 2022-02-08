use nix::{
    fcntl::{self, OFlag},
    sys::{
        stat::Mode,
        time::TimeSpec,
        timerfd::{ClockId, Expiration, TimerFd, TimerFlags, TimerSetTimeFlags},
    },
    unistd,
};
use std::{collections::HashMap, os::unix::io::AsRawFd, str, time::Duration};

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

    // Register polling for udev usb events
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

    // Register polling for kmsg/dmesg events
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

    // Register polling for a timer, for thermal sampling
    let timer = TimerFd::new(ClockId::CLOCK_MONOTONIC, TimerFlags::empty()).unwrap();
    timer
        .set(
            Expiration::Interval(TimeSpec::from_duration(Duration::from_secs(10))),
            TimerSetTimeFlags::empty(),
        )
        .unwrap();
    let timer_fd = timer.as_raw_fd();
    let mut timer_source = mio::unix::SourceFd(&timer_fd);
    poll.registry()
        .register(&mut timer_source, mio::Token(2), mio::Interest::READABLE)
        .unwrap();

    let mut udev_devices = HashMap::new();
    loop {
        poll.poll(&mut events, None).unwrap();

        for event in &events {
            if event.token() == mio::Token(0) && event.is_writable() {
                socket.clone().for_each(|x| {
                    if x.event_type() == udev::EventType::Add {
                        if let Some(event) = hp_vendor::peripheral_usb_type_a_event(x.syspath()) {
                            println!("{:#?}", event);
                            udev_devices.insert(x.syspath().to_owned(), event);
                        }
                    } else if x.event_type() == udev::EventType::Remove {
                        if let Some(mut event) = udev_devices.remove(x.syspath()) {
                            hp_vendor::event::remove_event(&mut event);
                            println!("{:#?}", event);
                        }
                    }
                });
            } else if event.token() == mio::Token(1) {
                let mut buf = [0; 1024];
                while let Ok(len) = unistd::read(fd, &mut buf) {
                    parse_kmsg(&buf[..len]);
                }
            } else if event.token() == mio::Token(2) {
                let mut buf = [0; 8];
                let _ = unistd::read(timer_fd, &mut buf);
                println!("timer");
            }
        }
    }
}
