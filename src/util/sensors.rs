use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    process::Command,
};

fn fan_to_rpm(value: u8) -> i64 {
    if value == 0xff {
        0
    } else {
        (7864320 / 32) / i64::from(value)
    }
}

// XXX use hwmon when implemented in driver
pub fn fan() -> Option<(i64, i64)> {
    Command::new("modprobe").arg("ec_sys").status().ok()?;
    let mut file = File::open("/sys/kernel/debug/ec/ec0/io").ok()?;
    file.seek(SeekFrom::Start(0x2e)).ok()?;
    let mut buf = [0; 2];
    file.read_exact(&mut buf).ok()?;
    Some((fan_to_rpm(buf[0]), fan_to_rpm(buf[1])))
}

pub fn thermal() -> Option<Vec<i64>> {
    let mut enumerator = udev::Enumerator::new().ok()?;
    enumerator.match_subsystem("hwmon").ok()?;
    enumerator.match_attribute("name", "acpitz").ok()?;
    let device = enumerator.scan_devices().ok()?.next()?;

    let mut temps = Vec::new();
    for i in 1..=6 {
        let value = device.attribute_value(format!("temp{}_input", i))?;
        temps.push(value.to_str()?.trim().parse().ok()?);
    }
    Some(temps)
}
