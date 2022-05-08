// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::event;

fn unix_time() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

fn format_unix_time(time: i64) -> String {
    OffsetDateTime::from_unix_timestamp(time)
        .unwrap()
        .format(&Rfc3339)
        .unwrap()
}

#[derive(Debug)]
pub struct Temps {
    pub cpu: i64,
    pub ext: i64,
    pub bat: i64,
    pub chg: i64,
    pub on_ac: bool,
    pub charging: bool,
    pub time: i64,
}

// Coppied from https://github.com/rust-lang/rust/pull/88582
// (Not in stable)
fn div_ceil(lhs: i64, rhs: i64) -> i64 {
    let d = lhs / rhs;
    let r = lhs % rhs;
    if (r > 0 && rhs > 0) || (r < 0 && rhs < 0) {
        d + 1
    } else {
        d
    }
}

fn percentiles(values: impl Iterator<Item = i64>) -> Vec<i64> {
    let mut values = values.collect::<Vec<_>>();
    values.sort_unstable();

    if values.is_empty() {
        return Vec::new();
    }

    let n = values.len() as i64;
    let percentile = |p| div_ceil(p * n, 100);

    vec![
        *values.first().unwrap(),
        percentile(10),
        percentile(20),
        percentile(30),
        percentile(40),
        percentile(50),
        percentile(60),
        percentile(70),
        percentile(80),
        percentile(90),
        *values.last().unwrap(),
    ]
}

// `temps` must be sorted by time, and non-empty
pub fn sumarize_temps(temps: &[Temps]) -> event::ThermalSummary {
    assert!(!temps.is_empty());

    let start_time = temps.first().unwrap().time;
    let end_time = temps.last().unwrap().time;
    // Assume system up one minute per sample
    // TODO: better method
    let system_up_time = temps.len() as i64 * 60;

    event::ThermalSummary {
        bat_zone_ac_charging_ptile: percentiles(
            temps
                .iter()
                .filter(|x| x.on_ac && x.charging)
                .map(|x| x.bat),
        ),
        bat_zone_ac_not_charging_ptile: percentiles(
            temps
                .iter()
                .filter(|x| x.on_ac && !x.charging)
                .map(|x| x.bat),
        ),
        bat_zone_dc_ptile: percentiles(temps.iter().filter(|x| !x.on_ac).map(|x| x.bat)),
        chg_zone_ptile: percentiles(temps.iter().map(|x| x.chg)),
        cpu_zone_ptile: percentiles(temps.iter().map(|x| x.cpu)),
        end_time: format_unix_time(end_time),
        ext_zone_ptile: percentiles(temps.iter().map(|x| x.ext)),
        num_samples: temps.len() as i64,
        start_time: format_unix_time(start_time),
        system_up_time,
    }
}

#[derive(Debug)]
pub struct Sensors {
    ac_device: udev::Device,
    bat_device: udev::Device,
    hwmon_acpi_device: udev::Device,
    hwmon_hp_device: udev::Device,
}

impl Sensors {
    pub fn new() -> Option<Self> {
        let mut enumerator = udev::Enumerator::new().ok()?;
        enumerator.match_subsystem("power_supply").ok()?;
        enumerator.match_attribute("type", "Mains").ok()?;
        let ac_device = enumerator.scan_devices().ok()?.next()?;

        let mut enumerator = udev::Enumerator::new().ok()?;
        enumerator.match_subsystem("power_supply").ok()?;
        enumerator.match_attribute("type", "Battery").ok()?;
        let bat_device = enumerator.scan_devices().ok()?.next()?;

        let mut enumerator = udev::Enumerator::new().ok()?;
        enumerator.match_subsystem("hwmon").ok()?;
        enumerator.match_attribute("name", "acpitz").ok()?;
        let hwmon_acpi_device = enumerator.scan_devices().ok()?.next()?;

        let mut enumerator = udev::Enumerator::new().ok()?;
        enumerator.match_subsystem("hwmon").ok()?;
        enumerator.match_attribute("name", "hp_vendor").ok()?;
        let hwmon_hp_device = enumerator.scan_devices().ok()?.next()?;

        Some(Self {
            ac_device,
            bat_device,
            hwmon_acpi_device,
            hwmon_hp_device,
        })
    }

    // Doesn't seem to be a way to clear udev sysattr cache
    pub fn update(&mut self) {
        fn reopen(device: &udev::Device) -> Option<udev::Device> {
            udev::Device::from_syspath(device.syspath()).ok()
        }
        if let Some(device) = reopen(&self.ac_device) {
            self.ac_device = device;
        }
        if let Some(device) = reopen(&self.bat_device) {
            self.bat_device = device;
        }
        if let Some(device) = reopen(&self.hwmon_acpi_device) {
            self.hwmon_acpi_device = device;
        }
        if let Some(device) = reopen(&self.hwmon_hp_device) {
            self.hwmon_hp_device = device;
        }
    }

    pub fn fan(&self) -> Option<i64> {
        self.hwmon_hp_device
            .attribute_value("fan1_input")?
            .to_str()?
            .trim()
            .parse()
            .ok()
    }

    pub fn thermal(&self) -> Option<Temps> {
        let temp_n = |n| {
            let value = self
                .hwmon_acpi_device
                .attribute_value(format!("temp{}_input", n))?;
            value.to_str()?.trim().parse().ok()
        };

        let on_ac = self.ac_device.attribute_value("online")?.to_str()?.trim() == "1";
        let charging = self.bat_device.attribute_value("status")?.to_str()?.trim() == "Charging";

        Some(Temps {
            cpu: temp_n(1)?,
            // Unused
            // gpu: temp_n(2)?,
            ext: temp_n(3)?,
            // Unused
            // loc: temp_n(4)?,
            bat: temp_n(5)?,
            chg: temp_n(6)?,
            on_ac,
            charging,
            time: unix_time(),
        })
    }
}
