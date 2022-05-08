// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

// XXX memory usage? Is there any danger remove events won't occur, and memory will grow?

use mio::{unix::SourceFd, Token};
use nix::{
    sys::{
        signal::{self, SigSet},
        signalfd::SignalFd,
        time::TimeSpec,
        timerfd::{ClockId, Expiration, TimerFd, TimerFlags, TimerSetTimeFlags},
    },
    unistd,
};
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{ErrorKind, Seek, SeekFrom},
    os::unix::{fs::OpenOptionsExt, io::AsRawFd},
    str,
    time::Duration,
};

use crate::{
    config::SamplingFrequency,
    db::{self, DB},
    event, util,
};

const TOKEN_SIGNAL: Token = Token(0);
const TOKEN_UDEV: Token = Token(1);
const TOKEN_KMSG: Token = Token(2);
const TOKEN_TIMER: Token = Token(3);

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

pub fn run() {
    // Get unique lock
    let _lock = util::lock::lock_file_or_panic("/var/hp-vendor/daemon.lock");

    let db = DB::open().unwrap();
    crate::exit_if_not_opted_in(&db);
    let mut insert_statement = db.prepare_queue_insert().unwrap();

    let mut poll = mio::Poll::new().unwrap();

    // Register polling for signals
    let mut mask = SigSet::empty();
    mask.add(signal::SIGTERM);
    mask.thread_block().unwrap();
    let signal = SignalFd::new(&mask).unwrap();
    poll.registry()
        .register(
            &mut SourceFd(&signal.as_raw_fd()),
            TOKEN_SIGNAL,
            mio::Interest::READABLE,
        )
        .unwrap();

    // Register polling for udev usb events
    let mut socket = udev::MonitorBuilder::new().unwrap().listen().unwrap();
    poll.registry()
        .register(
            &mut socket,
            TOKEN_UDEV,
            mio::Interest::READABLE | mio::Interest::WRITABLE,
        )
        .unwrap();

    // Register polling for kmsg/dmesg events
    let mut kmsg_file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/kmsg")
        .unwrap();
    kmsg_file.seek(SeekFrom::End(0)).unwrap();
    poll.registry()
        .register(
            &mut SourceFd(&kmsg_file.as_raw_fd()),
            TOKEN_KMSG,
            mio::Interest::READABLE,
        )
        .unwrap();

    // Register polling for a timer, for thermal sampling
    let timer = TimerFd::new(ClockId::CLOCK_MONOTONIC, TimerFlags::empty()).unwrap();
    timer
        .set(
            Expiration::Interval(TimeSpec::from_duration(Duration::from_secs(10))),
            TimerSetTimeFlags::empty(),
        )
        .unwrap();
    poll.registry()
        .register(
            &mut mio::unix::SourceFd(&timer.as_raw_fd()),
            TOKEN_TIMER,
            mio::Interest::READABLE,
        )
        .unwrap();

    let freqs = db.get_event_frequencies().unwrap();

    let mut udev_descs = crate::UdevDescs::new();
    for i in event::TelemetryEventType::iter() {
        if freqs.get(i) != SamplingFrequency::OnChange {
            continue;
        } else if let Some(crate::EventDesc::Udev(desc)) = crate::event(i) {
            udev_descs.insert(desc);
        }
    }

    let old = db
        .get_state(db::State::Frequency(SamplingFrequency::OnChange))
        .unwrap();

    let mut new = Vec::new();
    let mut enumerator = udev::Enumerator::new().unwrap();
    let mut udev_event_idx = Vec::new();
    for device in enumerator.scan_devices().unwrap() {
        let mut events = Vec::new();
        udev_descs.generate(&mut events, &device);
        if !events.is_empty() {
            new.extend_from_slice(&events);
            udev_event_idx.push((
                device.syspath().to_owned(),
                (new.len() - events.len())..new.len(),
            ));
        }
    }

    let mut diff = new.clone();
    event::diff(&mut diff, &old);
    for event in diff {
        insert_statement.execute(&event).unwrap();
        println!("{:#?}", event);
    }
    let ids = db
        .replace_state(db::State::Frequency(SamplingFrequency::OnChange), &new)
        .unwrap();
    let mut udev_devices = udev_event_idx
        .into_iter()
        .map(|(syspath, range)| (syspath, ids[range].to_owned()))
        .collect::<HashMap<_, _>>();

    let mut sensors = util::Sensors::new();
    if sensors.is_none() {
        eprintln!("Error: Failed to intitialize `Sensors`");
    }

    let mut events = mio::Events::with_capacity(1024);
    loop {
        loop {
            match poll.poll(&mut events, None) {
                Ok(()) => {
                    break;
                }
                Err(err) => {
                    if err.kind() != ErrorKind::Interrupted {
                        panic!("Mio polling error {}", err);
                    }
                }
            }
        }

        for event in &events {
            match event.token() {
                TOKEN_SIGNAL => {
                    println!("SIGTERM");
                    return;
                }
                TOKEN_UDEV => {
                    socket.clone().for_each(|x| {
                        if x.event_type() == udev::EventType::Add {
                            let mut events = Vec::new();
                            udev_descs.generate(&mut events, &x);
                            for event in &events {
                                println!("{:#?}", event);
                                insert_statement.execute(event).unwrap();
                            }
                            if !events.is_empty() {
                                let ids = db.replace_state(db::State::Ids(&[]), &events).unwrap();
                                udev_devices.insert(x.syspath().to_owned(), ids);
                            } else {
                                udev_devices.remove(x.syspath());
                            }
                        } else if x.event_type() == udev::EventType::Remove {
                            if let Some(ids) = udev_devices.remove(x.syspath()) {
                                let events = db.get_state(db::State::Ids(&ids)).unwrap();
                                for mut event in events {
                                    crate::event::remove_event(&mut event);
                                    println!("{:#?}", event);
                                    insert_statement.execute(&event).unwrap();
                                }
                                db.replace_state(db::State::Ids(&ids), &[]).unwrap();
                            }
                        } else if x.event_type() == udev::EventType::Change {
                            if let Some(ids) = udev_devices.remove(x.syspath()) {
                                let old = db.get_state(db::State::Ids(&ids)).unwrap();
                                let mut new = Vec::new();
                                udev_descs.generate(&mut new, &x);
                                let mut diff = new.clone();
                                event::diff(&mut diff, &old);
                                for event in diff {
                                    insert_statement.execute(&event).unwrap();
                                    println!("{:#?}", event);
                                }
                                if !events.is_empty() {
                                    let ids = db.replace_state(db::State::Ids(&ids), &new).unwrap();
                                    udev_devices.insert(x.syspath().to_owned(), ids);
                                }
                            }
                        }
                    });
                }
                TOKEN_KMSG => {
                    let mut buf = [0; 1024];
                    while let Ok(len) = unistd::read(kmsg_file.as_raw_fd(), &mut buf) {
                        parse_kmsg(&buf[..len]);
                    }
                }
                TOKEN_TIMER => {
                    // println!("timer");
                    let mut buf = [0; 8];
                    let _ = unistd::read(timer.as_raw_fd(), &mut buf);
                    if let Some(sensors) = &mut sensors {
                        sensors.update();
                        if let Some(rpm) = sensors.fan() {
                            // println!("Fan: {} RPM", rpm);
                        }
                        if let Some(temps) = sensors.thermal() {
                            // println!("Temps: {:?}", temps);
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}
