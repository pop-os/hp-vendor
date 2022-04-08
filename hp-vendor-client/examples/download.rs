// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: MPL-2.0

use std::{env, fs, io};

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap();
    let format = args.next().unwrap();
    let mut file = fs::File::create(path).unwrap();
    let mut download = hp_vendor_client::download(format.parse().unwrap()).unwrap();
    println!("length: {}", download.length());
    io::copy(&mut download, &mut file).unwrap();
}
