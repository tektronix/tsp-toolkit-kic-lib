use crate::new::InstrumentInfo;

#[derive(Debug, Clone)]
pub enum Event {
    Connected(InstrumentInfo),
    WriteProgress(Progress),
    WriteComplete,
    FwProgress(Progress),
    FwComplete,
    ScriptProgress(Progress),
    ScriptComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    written: usize,
    total: usize,
}

impl Progress {
    #[must_use]
    pub const fn new(total: usize) -> Self {
        Self { written: 0, total }
    }

    pub fn add_progress(&mut self, written: usize) {
        self.written = self.written.saturating_add(written);
    }
}
