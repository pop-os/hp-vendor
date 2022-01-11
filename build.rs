use convert_case::{Case, Casing};
use serde_json::{Map, Value};
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

fn main() {
    println!("cargo:rerun-if-changed=event_package.json");

    let json_str = fs::read_to_string("event_package.json").unwrap();
    let root: Map<String, Value> = serde_json::from_str(&json_str).unwrap();
    let definitions = root.get("definitions").unwrap().as_object().unwrap();
    let any_event = definitions
        .get("AnyTelemetryEvent")
        .unwrap()
        .as_object()
        .unwrap();
    let properties = any_event.get("properties").unwrap().as_object().unwrap();
    let properties = properties
        .iter()
        .map(|(k, v)| {
            let variant = k.to_case(Case::UpperCamel);
            let ref_ = v
                .as_object()
                .unwrap()
                .get("$ref")
                .unwrap()
                .as_str()
                .unwrap();
            let struct_ = ref_.strip_prefix("#/definitions/").unwrap();
            let struct_ = struct_.replace("NVMES", "Nvmes"); // XXX Why? Better?
            (variant, struct_)
        })
        .collect::<Vec<_>>();

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let mut path = PathBuf::from(out_dir);
    path.push("event_enum.rs");
    let mut file = File::create(path).unwrap();

    // Generate a `AnyTelemetryEventEnum` enum
    writeln!(file, "#[derive(Debug, Deserialize, Serialize)]").unwrap();
    writeln!(file, "#[serde(rename_all = \"snake_case\")]").unwrap();
    writeln!(file, "pub enum AnyTelemetryEventEnum {{").unwrap();
    for (variant, struct_) in properties.iter() {
        writeln!(file, "    {}({}),", variant, &struct_).unwrap();
    }
    writeln!(file, "}}").unwrap();

    // Implement `From<T> for AnyTelemetryEventEnum` for every type wrapper by enum
    for (variant, struct_) in properties.iter() {
        writeln!(file, "impl From<{}> for AnyTelemetryEventEnum {{", struct_).unwrap();
        writeln!(file, "    fn from(value: {}) -> Self {{", struct_).unwrap();
        writeln!(file, "        AnyTelemetryEventEnum::{}(value)", variant).unwrap();
        writeln!(file, "    }}").unwrap();
        writeln!(file, "}}").unwrap();
    }

    // Define `TelemetryEventType` enum
    writeln!(file, "#[derive(Debug, Clone, Copy)]").unwrap();
    writeln!(file, "pub enum TelemetryEventType {{").unwrap();
    for (variant, _) in properties.iter() {
        writeln!(file, "    {},", variant).unwrap();
    }
    writeln!(file, "}}").unwrap();

    // Generate `TelemetryEventType::iter()` method to iterate over variants
    writeln!(file, "impl TelemetryEventType {{").unwrap();
    writeln!(file, "    pub fn iter() -> impl Iterator<Item=Self> {{").unwrap();
    writeln!(file, "        static VARIANTS: &[TelemetryEventType] = &[").unwrap();
    for (variant, _) in properties.iter() {
        writeln!(file, "            TelemetryEventType::{},", variant).unwrap();
    }
    writeln!(file, "        ];").unwrap();
    writeln!(file, "        VARIANTS.iter().copied()").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();

    // Generate function from `AnyTelemetryEventEnum` to `AnyTelemetryEventEnum`
    writeln!(file, "impl AnyTelemetryEventEnum {{").unwrap();
    writeln!(file, "    fn type_(&self) -> TelemetryEventType {{").unwrap();
    writeln!(file, "        match self {{").unwrap();
    for (variant, _) in properties.iter() {
        writeln!(
            file,
            "            AnyTelemetryEventEnum::{0}(_) => TelemetryEventType::{0},",
            variant
        )
        .unwrap();
    }
    writeln!(file, "        }}").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();
}
