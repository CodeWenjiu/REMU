use super::Emu;

impl Emu {
    pub fn instruction_trace(&mut self, enable: bool) {
        self.instruction_trace_enable = enable;
    }
}