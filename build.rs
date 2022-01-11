use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, TokenStreamExt};
use serde_json::Value;
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

fn main() {
    println!("cargo:rerun-if-changed=event_package.json");

    let json_str = fs::read_to_string("event_package.json").unwrap();
    let root: Value = serde_json::from_str(&json_str).unwrap();
    let properties = root
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
        .collect::<Vec<_>>();

    let mut tokens = TokenStream::new();

    // Generate a `AnyTelemetryEventEnum` enum
    let variants = properties.iter().map(|(variant, struct_)| {
        quote! {#variant(#struct_)}
    });
    tokens.append_all(quote! {
        #[derive(Debug, Deserialize, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum AnyTelemetryEventEnum {
            #(#variants),*
        }
    });

    // Implement `From<T> for AnyTelemetryEventEnum` for every type wrapper by enum
    for (variant, struct_) in properties.iter() {
        tokens.append_all(quote! {
            impl From<#struct_> for AnyTelemetryEventEnum {
                fn from(value: #struct_) -> Self {
                    AnyTelemetryEventEnum::#variant(value)
                }
            }
        });
    }

    // Define `TelemetryEventType` enum
    let variants = properties.iter().map(|(variant, _)| variant);
    tokens.append_all(quote! {
        #[derive(Debug, Clone, Copy)]
        pub enum TelemetryEventType {
            #(#variants),*
        }
    });

    // Generate `TelemetryEventType::iter()` method to iterate over variants
    let variants = properties
        .iter()
        .map(|(variant, _)| quote! {TelemetryEventType::#variant});
    tokens.append_all(quote! {
        impl TelemetryEventType {
            pub fn iter() -> impl Iterator<Item=Self> {
                static VARIANTS: &[TelemetryEventType] = &[
                    #(#variants),*
                ];
                VARIANTS.iter().copied()
            }
        }
    });

    // Generate function from `AnyTelemetryEventEnum` to `AnyTelemetryEventEnum`
    let cases = properties.iter().map(
        |(variant, _)| quote!(AnyTelemetryEventEnum::#variant(_) => TelemetryEventType::#variant),
    );
    tokens.append_all(quote! {
        impl AnyTelemetryEventEnum {
            fn type_(&self) -> TelemetryEventType {
                match self {
                    #(#cases),*
                }
            }
        }
    });

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let mut path = PathBuf::from(out_dir);
    path.push("event_enum.rs");
    let mut file = File::create(path).unwrap();
    writeln!(file, "{}", tokens).unwrap();
}
