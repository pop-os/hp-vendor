use hp_vendor::event::{Event, TelemetryEventType};

#[test]
fn validate() {
    let mut events = Vec::new();
    for i in TelemetryEventType::iter() {
        if let Some(event) = hp_vendor::event(i) {
            event.generate(&mut events);
        }
    }
    let value = serde_json::to_value(Event::new(events)).unwrap();

    let mut scope = valico::json_schema::Scope::new();
    let schema_json: serde_json::Value =
        serde_json::from_str(include_str!("../UploadEventPackageRequestModel.json")).unwrap();
    let schema = scope.compile_and_return(schema_json, false).unwrap();
    let result = schema.validate(&value);
    assert!(result.is_valid(), "{:#?}", result);
}
