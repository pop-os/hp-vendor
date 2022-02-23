use std::{
    io::{self, Write},
    process,
};

use crate::{api::Api, db::DB, event::DeviceOSIds};

pub fn run(arg: Option<&str>) {
    let locale = arg.unwrap_or_else(|| {
        eprintln!("Usage: hp-vendor consent <locale>");
        process::exit(1)
    });

    let db = DB::open().unwrap();
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = DeviceOSIds::new(os_install_id).unwrap();

    // XXX show existing consent

    let api = Api::new(ids).unwrap();
    let purposes = api.purposes(locale).unwrap();
    // XXX multiple?
    let purpose = &purposes[0];

    println!("Purpose: {}", purpose.statement);
    print!("Agree? [yN]: ");
    io::stdout().lock().flush().unwrap();
    let mut answer = String::new();
    io::stdin().read_line(&mut answer).unwrap();
    if answer.trim() == "y" {
        let resp = api.consent(&purpose.locale, &purpose.version).unwrap();
        println!("{:?}", resp);
        // XXX set consent
    }
}
