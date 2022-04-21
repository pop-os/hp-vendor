// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

#[derive(Debug)]
pub struct Temps {
    pub cpu: i64,
    pub gpu: i64,
    pub ext: i64,
    pub loc: i64,
    pub bat: i64,
    pub chg: i64,
}

pub fn fan() -> Option<i64> {
    let mut enumerator = udev::Enumerator::new().ok()?;
    enumerator.match_subsystem("hwmon").ok()?;
    enumerator.match_attribute("name", "hp_vendor").ok()?;
    let device = enumerator.scan_devices().ok()?.next()?;

    device
        .attribute_value("fan1_input")?
        .to_str()?
        .trim()
        .parse()
        .ok()
}

pub fn thermal() -> Option<Temps> {
    let mut enumerator = udev::Enumerator::new().ok()?;
    enumerator.match_subsystem("hwmon").ok()?;
    enumerator.match_attribute("name", "acpitz").ok()?;
    let device = enumerator.scan_devices().ok()?.next()?;

    let temp_n = |n| {
        let value = device.attribute_value(format!("temp{}_input", n))?;
        value.to_str()?.trim().parse().ok()
    };

    Some(Temps {
        cpu: temp_n(1)?,
        gpu: temp_n(2)?,
        ext: temp_n(3)?,
        loc: temp_n(4)?,
        bat: temp_n(5)?,
        chg: temp_n(6)?,
    })
}
