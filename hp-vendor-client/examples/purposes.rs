// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: MPL-2.0

use std::env;

fn main() {
    let arg = env::args().skip(1).next();
    let fetch = arg.as_deref() != Some("--no-fetch");
    println!("{:#?}", hp_vendor_client::purposes(fetch).unwrap());
}
