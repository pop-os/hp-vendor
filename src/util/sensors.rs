// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

#[derive(Debug)]
pub struct Temps {
    pub cpu: i64,
    pub ext: i64,
    pub bat: i64,
    pub chg: i64,
    pub on_ac: bool,
    pub charging: bool,
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
        })
    }
}
