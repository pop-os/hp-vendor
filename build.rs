use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span};
use quote::quote;
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
            let variant = Ident::new(&variant, Span::call_site());
            let struct_ = Ident::new(&struct_, Span::call_site());
            (variant, struct_)
        })
        .collect::<Vec<_>>();

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let mut path = PathBuf::from(out_dir);
    path.push("event_enum.rs");
    let mut file = File::create(path).unwrap();

    // Generate a `AnyTelemetryEventEnum` enum
    let variants = properties.iter().map(|(variant, struct_)| {
        quote! {#variant(#struct_)}
    });
    writeln!(
        file,
        "{}",
        quote! {
            #[derive(Debug, Deserialize, Serialize)]
            #[serde(rename_all = "snake_case")]
            pub enum AnyTelemetryEventEnum {
                #(#variants),*
            }
        }
    )
    .unwrap();

    // Implement `From<T> for AnyTelemetryEventEnum` for every type wrapper by enum
    for (variant, struct_) in properties.iter() {
        writeln!(
            file,
            "{}",
            quote! {
                impl From<#struct_> for AnyTelemetryEventEnum {
                    fn from(value: #struct_) -> Self {
                        AnyTelemetryEventEnum::#variant(value)
                    }
                }
            }
        )
        .unwrap();
    }

    // Define `TelemetryEventType` enum
    let variants = properties.iter().map(|(variant, _)| variant);
    writeln!(
        file,
        "{}",
        quote! {
            #[derive(Debug, Clone, Copy)]
            pub enum TelemetryEventType {
                #(#variants),*
            }
        }
    )
    .unwrap();

    // Generate `TelemetryEventType::iter()` method to iterate over variants
    let variants = properties.iter().map(|(variant, _)| {
        quote! {TelemetryEventType::#variant}
    });
    writeln!(
        file,
        "{}",
        quote! {
            impl TelemetryEventType {
                pub fn iter() -> impl Iterator<Item=Self> {
                    static VARIANTS: &[TelemetryEventType] = &[
                        #(#variants),*
                    ];
                    VARIANTS.iter().copied()
                }
            }
        }
    )
    .unwrap();

    // Generate function from `AnyTelemetryEventEnum` to `AnyTelemetryEventEnum`
    let cases = properties.iter().map(
        |(variant, _)| quote!(AnyTelemetryEventEnum::#variant(_) => TelemetryEventType::#variant),
    );
    writeln!(
        file,
        "{}",
        quote! {
            impl AnyTelemetryEventEnum {
                fn type_(&self) -> TelemetryEventType {
                    match self {
                        #(#cases),*
                    }
                }
            }
        }
    )
    .unwrap();
}
