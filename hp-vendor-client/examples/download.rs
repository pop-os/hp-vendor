use std::{env, fs};

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap();
    let format = args.next().unwrap();
    let file = fs::File::create(path).unwrap();
    hp_vendor_client::download(file, format.parse().unwrap()).unwrap();
}
