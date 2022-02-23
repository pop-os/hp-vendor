use crate::{
    api::{self, Api},
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
    let ids = event::DeviceOSIds::new(os_install_id).unwrap();
    let freqs = db.get_event_frequencies().unwrap();

    // TODO: handle frequencies other than daily
    let old = db
        .get_state(db::State::Frequency(SamplingFrequency::Daily))
        .unwrap();

    let new = crate::events(&freqs, SamplingFrequency::Daily);
    let mut diff = new.clone();
    event::diff(&mut diff, &old);

    let mut insert_statement = db.prepare_queue_insert().unwrap();
    for event in diff {
        insert_statement.execute(&event).unwrap();
    }
    db.replace_state(db::State::Frequency(SamplingFrequency::Daily), &new)
        .unwrap();

    let api = if arg != Some("--dequeue-no-upload") {
        match Api::new(ids.clone()) {
            Ok(api) => Some(api),
            Err(err) => panic!("Failed to authenticate with server: {}", err),
        }
    } else {
        None
    };

    let (queued_ids, queued) = db.get_queued().unwrap();
    let mut events = event::Events::new(consent, ids, &[]);
    for (chunk_ids, chunk) in queued_ids.chunks(100).zip(queued.chunks(100)) {
        events.data = chunk;

        println!("{}", events.to_json_pretty());

        if let Some(api) = &api {
            let mut start = 0;
            let mut end = chunk.len();
            while start < chunk.len() {
                events.data = &chunk[start..end];
                match api.upload(&events) {
                    Ok(res) => {
                        println!("{:#?}", res);
                        db.remove_queued(&chunk_ids[start..end]).unwrap();
                        start = end;
                        end = chunk.len();
                    }
                    Err(err) => {
                        if err.is::<api::PayloadSizeError>() {
                            // Try to transmit fewer events
                            end = start + (end - start) / 2;
                        } else {
                            panic!("Failed to upload: {}", err);
                        }
                    }
                }
            }
        } else {
            db.remove_queued(chunk_ids).unwrap();
        }
    }
}
