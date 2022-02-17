use crate::{
    api::Api,
    config::SamplingFrequency,
    db::{self, DB},
    event, util,
};

pub fn run(arg: Option<&str>) {
    // Get unique lock
    let _lock = util::lock_file_or_panic("/var/hp-vendor/daily.lock");

    // XXX handle db errors?
    let db = DB::open().unwrap();
    db.update_event_types().unwrap();

    // TODO set consent correctly, and check its value
    let consent = match db.get_consent().unwrap() {
        Some(consent) => consent,
        None => {
            let consent = event::DataCollectionConsent {
                opted_in_level: String::new(),
                version: String::new(),
            };
            db.set_consent(Some(&consent)).unwrap();
            consent
        }
    };
    let os_install_id = db.get_os_install_id().unwrap();
    let ids = event::DeviceOSIds::new(os_install_id);
    let freqs = db.get_event_frequencies().unwrap();

    // TODO: handle frequencies other than daily
    let old = db
        .get_state(db::State::Frequency(SamplingFrequency::Daily))
        .unwrap();

    let new = crate::events(&freqs, SamplingFrequency::Daily);
    let mut diff = new.clone();
    event::diff(&mut diff, &old);

    diff.extend_from_slice(&db.get_queued(true).unwrap());

    let events = event::Events::new(consent, ids.clone(), &diff);
    println!("{}", events.to_json_pretty());

    if arg != Some("--dequeue-no-upload") {
        let api = Api::new(ids).unwrap();
        println!("{:#?}", api.upload(&events).unwrap());
    }

    db.clear_queued().unwrap();
    db.replace_state(db::State::Frequency(SamplingFrequency::Daily), &new)
        .unwrap();
}
