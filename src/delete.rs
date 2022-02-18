use crate::{api::Api, db::DB, event::DeviceOSIds};

pub fn run() {
    let db = DB::open().unwrap();
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = DeviceOSIds::new(os_install_id).unwrap();

    let api = Api::new(ids).unwrap();

    api.delete().unwrap();
}
