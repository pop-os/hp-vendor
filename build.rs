use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span};
use quote::quote;
use serde_json::Value;
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

fn main() {
    println!("cargo:rerun-if-changed=UploadEventPackageRequestModel.json");

    let json_str = fs::read_to_string("UploadEventPackageRequestModel.json").unwrap();
    let root: Value = serde_json::from_str(&json_str).unwrap();
    let (variants, structs): (Vec<_>, Vec<_>) = root
        .pointer("/definitions/AnyTelemetryEvent/properties")
        .unwrap()
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| {
            let variant = k.to_case(Case::UpperCamel);
            let ref_ = v.pointer("/$ref").unwrap().as_str().unwrap();
            let struct_ = ref_.strip_prefix("#/definitions/").unwrap();
            let struct_ = struct_.replace("NVMES", "Nvmes"); // XXX Why? Better?
            (
                Ident::new(&variant, Span::call_site()),
                Ident::new(&struct_, Span::call_site()),
            )
        })
        .unzip();

    let tokens = quote! {
        // Generate a `TelemetryEvent` enum
        #[derive(Debug, Deserialize, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum TelemetryEvent {
            #(#variants(#structs)),*
        }

        // Implement `From<T> for TelemetryEvent` for every type wrapper by enum
        #(
            impl From<#structs> for TelemetryEvent {
                fn from(value: #structs) -> Self {
                    TelemetryEvent::#variants(value)
                }
            }
        )*

        // Define `TelemetryEventType` enum
        #[derive(Debug, Clone, Copy)]
        pub enum TelemetryEventType {
            #(#variants),*
        }

        // Generate `TelemetryEventType::iter()` method to iterate over variants
        impl TelemetryEventType {
            pub fn iter() -> impl Iterator<Item=Self> {
                static VARIANTS: &[TelemetryEventType] = &[
                    #(TelemetryEventType::#variants),*
                ];
                VARIANTS.iter().copied()
            }
        }

        // Generate function from `TelemetryEvent` to `TelemetryEvent`
        impl TelemetryEvent {
            fn type_(&self) -> TelemetryEventType {
                match self {
                    #(TelemetryEvent::#variants(_) => TelemetryEventType::#variants),*
                }
            }
        }
    };

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let mut path = PathBuf::from(out_dir);
    path.push("event_enum.rs");
    let mut file = File::create(path).unwrap();
    writeln!(file, "{}", tokens).unwrap();
}
