/// State template
pub struct State {}

impl State {
    pub fn new() -> Self {
        State {}
    }

    pub fn hello(&self) {
        tracing::info!("hello state");
    }
}
