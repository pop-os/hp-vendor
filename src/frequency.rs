use std::{collections::HashMap, iter::FromIterator};

use crate::event::TelemetryEventType;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Frequency {
    Daily,
    Trigger,
}

impl Frequency {
    pub fn to_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Trigger => "trigger",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "daily" => Some(Self::Daily),
            "trigger" => Some(Self::Trigger),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Frequencies(HashMap<TelemetryEventType, Frequency>);

impl Frequencies {
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (TelemetryEventType, Frequency)> + 'a {
        self.0.iter().map(|(k, v)| (k.clone(), v.clone()))
    }

    pub fn from_iter_or_default<T: Iterator<Item = (TelemetryEventType, Frequency)>>(
        iter: T,
    ) -> Self {
        let mut freqs = HashMap::from_iter(iter);
        for i in TelemetryEventType::iter() {
            freqs.entry(i).or_insert(default_frequency(i));
        }
        Self(freqs)
    }

    pub fn get(&self, type_: TelemetryEventType) -> Frequency {
        // NOTE: must statically ensure every variant is in this
        self.0.get(&type_).unwrap().clone()
    }
}

impl Default for Frequencies {
    fn default() -> Self {
        Self(
            TelemetryEventType::iter()
                .map(|i| (i, default_frequency(i)))
                .collect(),
        )
    }
}

fn default_frequency(type_: TelemetryEventType) -> Frequency {
    use Frequency::*;
    use TelemetryEventType::*;

    match type_ {
        HwBaseBoard => Daily,
        HwBattery => Daily,
        HwBatteryUsage => Daily, // XXX make trigger based
        HwCoolingFanCyclesSummary => Daily,
        HwDisplay => Daily,
        HwGraphicsCard => Daily,
        HwMemoryPhysical => Daily,
        HwNetworkCard => Daily,
        HwNvmeSmartLog => Daily,
        HwNvmeStorageLogical => Daily,
        HwNvmeStoragePhysical => Daily,
        HwPeripheralAudioPort => Daily,
        HwPeripheralHdmi => Daily,
        HwPeripheralSimCard => Daily,
        HwPeripheralUsbTypeA => Trigger,
        HwPeripheralUsbTypeC => Daily,
        HwPeripheralUsbTypeCDisplayPort => Daily,
        HwProcessor => Daily,
        HwSystem => Daily,
        HwThermalSummary => Daily,
        HwTpm => Daily,
        SwDriver => Daily,
        SwFirmware => Daily,
        SwLinuxDriverCrash => Daily,
        SwLinuxKernel => Daily,
        SwOperatingSystem => Daily,
    }
}
