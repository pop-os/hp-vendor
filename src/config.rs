use serde::{Deserialize, Serialize};

schemafy::schemafy!("DataConfigResponseModel.json");

impl Copy for SamplingFrequency {}
