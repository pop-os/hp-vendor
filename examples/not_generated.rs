use hp_vendor::event;

fn main() {
    for i in event::TelemetryEventType::iter() {
        if hp_vendor::event(i).is_none() {
            println!("{:?}", i);
        }
    }
}
