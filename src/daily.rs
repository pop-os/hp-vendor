use crate::{
    db::{self, DB},
    event,
    frequency::Frequency,
    util,
};

pub fn run() {
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
    let freqs = db.get_event_frequencies().unwrap();

    // TODO: handle frequencies other than daily
    let old = db
        .get_state(db::State::Frequency(Frequency::Daily))
        .unwrap();

    let new = crate::events(&freqs, Frequency::Daily);
    let mut diff = new.clone();
    event::diff(&mut diff, &old);

    diff.extend_from_slice(&db.get_queued(true).unwrap());

    let events = event::Events::new(consent, diff);
    println!("{}", events.to_json_pretty());

    /*
    let client = reqwest::blocking::Client::new();
    let token = hp_vendor::api::TokenRequest::new()
        .send(&client)
        .unwrap()
        .token;
    println!("{:#?}", events.send(&client, &token).unwrap());
    */

    db.clear_queued().unwrap();
    db.replace_state(db::State::Frequency(Frequency::Daily), &new)
        .unwrap();
}
