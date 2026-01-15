remu_macro::mod_flat!(memory);

/// State template
pub struct State {}

impl State {
    pub fn new(opt: StateOption) -> Self {
        tracing::info!("{:?}", opt);

        State {}
    }

    pub fn hello(&self) {
        tracing::info!("hello state");
    }
}
