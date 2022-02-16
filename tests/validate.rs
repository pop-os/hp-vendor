use hp_vendor::event::{DataCollectionConsent, Events};

#[test]
fn validate() {
    let consent = DataCollectionConsent {
        opted_in_level: String::new(),
        version: String::new(),
    };
    let events = hp_vendor::all_events();

    let mut scope = valico::json_schema::Scope::new();
    let schema_json: serde_json::Value =
        serde_json::from_str(include_str!("../DataUploadRequestModel.json")).unwrap();
    let schema = scope.compile_and_return(schema_json, false).unwrap();
    for chunk in events.chunks(100) {
        let value = serde_json::to_value(Events::new(consent.clone(), chunk)).unwrap();
        let result = schema.validate(&value);
        assert!(result.is_valid(), "{:#?}", result);
    }
}
