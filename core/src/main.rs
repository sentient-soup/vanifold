// Vanifold core daemon. The spine (MQTT -> discovery -> registry -> API -> history)
// is specified in docs/; implementation lands in that order.
fn main() {
    println!("vanifold-core {}", env!("CARGO_PKG_VERSION"));
}
