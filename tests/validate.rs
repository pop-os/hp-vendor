#[test]
fn validate() {
    let event = hp_vendor::event::Event {
        data_header: hp_vendor::data_header(),
        data: hp_vendor::event::TelemetryEventType::iter()
            .filter_map(hp_vendor::event)
            .map(|x| x.generate())
            .collect(),
    };
    let value = serde_json::to_value(event).unwrap();

    let mut scope = valico::json_schema::Scope::new();
    let schema_json: serde_json::Value =
        serde_json::from_str(include_str!("../UploadEventPackageRequestModel.json")).unwrap();
    let schema = scope.compile_and_return(schema_json, false).unwrap();
    let result = schema.validate(&value);
    assert!(result.is_valid(), "{:#?}", result);
}
