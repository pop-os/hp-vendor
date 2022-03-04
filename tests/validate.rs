use hp_vendor::event::{DataCollectionConsent, DeviceOSIds, Events};

#[test]
fn validate() {
    let consents = vec![DataCollectionConsent {
        country: String::new(),
        locale: String::new(),
        purpose_id: String::new(),
        version: String::new(),
    }];
    let ids = DeviceOSIds::new(uuid::Uuid::new_v4().to_string()).unwrap();
    let events = hp_vendor::all_events();

    let mut scope = valico::json_schema::Scope::new();
    let schema_json: serde_json::Value =
        serde_json::from_str(include_str!("../DataUploadRequestModel.json")).unwrap();
    let schema = scope.compile_and_return(schema_json, false).unwrap();
    for chunk in events.chunks(100) {
        let value =
            serde_json::to_value(Events::new(consents.clone(), ids.clone(), chunk)).unwrap();
        let result = schema.validate(&value);
        assert!(result.is_valid(), "{:#?}", result);
    }
}
