use serde::{Deserialize, Serialize};

schemafy::schemafy!("DataConfigResponseModel.json");

impl Copy for SamplingFrequency {}

// TODO: fails to parse if one is removed, right?
#[derive(serde::Deserialize)]
pub struct Config {
    #[allow(dead_code)]
    sampling_frequency: SamplingFrequencyModel,
}
