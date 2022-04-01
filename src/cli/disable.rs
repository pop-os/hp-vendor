use crate::{db::DB, util};

pub fn run() {
    let db = DB::open().unwrap();
    db.set_consent(None).unwrap();
    util::systemd::disable_services_and_timers();
}
