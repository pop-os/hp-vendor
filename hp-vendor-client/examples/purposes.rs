use std::env;

fn main() {
    let locale = env::args().skip(1).next().unwrap();
    println!("{:#?}", hp_vendor_client::purposes(&locale).unwrap());
}
