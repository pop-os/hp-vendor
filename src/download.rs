use crate::{api::Api, db::DB, event::DeviceOSIds};
use std::io::{self, Write};

pub fn run(arg: Option<&str>) {
    let db = DB::open().unwrap();
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = DeviceOSIds::new(os_install_id).unwrap();

    let api = Api::new(ids).unwrap();

    let zip = arg == Some("--zip");
    let res = api.download(zip).unwrap();
    io::stdout().write(&res).unwrap();
}
