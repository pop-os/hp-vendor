// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{collections::HashMap, iter::FromIterator};

use crate::{config::SamplingFrequency, event::TelemetryEventType};

impl SamplingFrequency {
    pub fn to_str(self) -> &'static str {
        match self {
            Self::OnTrigger => "on_trigger",
            Self::OnChange => "on_change",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "on_trigger" => Some(Self::OnTrigger),
            "on_change" => Some(Self::OnChange),
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Frequencies(HashMap<TelemetryEventType, SamplingFrequency>);

impl Frequencies {
    pub fn iter<'a>(
        &'a self,
    ) -> impl Iterator<Item = (TelemetryEventType, SamplingFrequency)> + 'a {
        self.0.iter().map(|(k, v)| (k.clone(), v.clone()))
    }

    pub fn from_iter_or_default<T: Iterator<Item = (TelemetryEventType, SamplingFrequency)>>(
        iter: T,
    ) -> Self {
        let mut freqs = HashMap::from_iter(iter);
        for i in TelemetryEventType::iter() {
            freqs.entry(i).or_insert(default_frequency(i));
        }
        Self(freqs)
    }

    pub fn get(&self, type_: TelemetryEventType) -> SamplingFrequency {
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

fn default_frequency(type_: TelemetryEventType) -> SamplingFrequency {
    use SamplingFrequency::*;

    match type_ {
        TelemetryEventType::HwBaseBoard => Daily,
        TelemetryEventType::HwBattery => Daily,
        TelemetryEventType::HwBatteryLife => Daily, // XXX make trigger based
        TelemetryEventType::HwCoolingFanCyclesSummary => Daily,
        TelemetryEventType::HwDisplay => OnChange,
        TelemetryEventType::HwGraphicsCard => Daily,
        TelemetryEventType::HwMemoryPhysical => Daily,
        TelemetryEventType::HwNetworkCard => Daily,
        TelemetryEventType::HwNvmeSmartLog => Daily,
        TelemetryEventType::HwNvmeStorageLogical => Daily,
        TelemetryEventType::HwNvmeStoragePhysical => Daily,
        TelemetryEventType::HwPeripheralAudioPort => Daily,
        TelemetryEventType::HwPeripheralUsb => OnChange,
        TelemetryEventType::HwPeripheralSimCard => Daily,
        TelemetryEventType::HwProcessor => Daily,
        TelemetryEventType::HwSystem => Daily,
        TelemetryEventType::HwThermalSummary => Daily,
        TelemetryEventType::HwTpm => Daily,
        TelemetryEventType::SwBootPerformance => Daily,
        TelemetryEventType::SwDriver => Daily,
        TelemetryEventType::SwFirmware => Daily,
        TelemetryEventType::SwLinuxDriverCrash => Daily,
        TelemetryEventType::SwLinuxKernel => Daily,
        TelemetryEventType::SwOperatingSystem => Daily,
    }
}
