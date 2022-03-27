// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{event::TelemetryEventType, frequency::Frequencies};

schemafy::schemafy!("DataConfigResponseModel.json");

impl Copy for SamplingFrequency {}
impl Eq for SamplingFrequency {}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum Freq {
    Known(SamplingFrequency),
    Unknown(String),
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
enum Type {
    Known(TelemetryEventType),
    Unknown(String),
}

// TODO: fails to parse if one is removed, right?
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[allow(dead_code)]
    sampling_frequency: HashMap<Type, Freq>,
    // sampling_frequency: SamplingFrequencyModel,
}

impl Config {
    pub fn frequencies(&self) -> Frequencies {
        // TODO: warn unknown, and missing
        let iter = self.sampling_frequency.iter().filter_map(|i| {
            if let (Type::Known(type_), Freq::Known(freq)) = i {
                Some((*type_, *freq))
            } else {
                None
            }
        });
        Frequencies::from_iter_or_default(iter)
    }
}
