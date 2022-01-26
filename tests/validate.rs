use hp_vendor::event::Events;

#[test]
fn validate() {
    let value = serde_json::to_value(Events::new(hp_vendor::all_events())).unwrap();

    let mut scope = valico::json_schema::Scope::new();
    let schema_json: serde_json::Value =
        serde_json::from_str(include_str!("../UploadEventPackageRequestModel.json")).unwrap();
    let schema = scope.compile_and_return(schema_json, false).unwrap();
    let result = schema.validate(&value);
    assert!(result.is_valid(), "{:#?}", result);
}
