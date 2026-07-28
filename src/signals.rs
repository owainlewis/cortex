use signal_hook::{
    consts::{SIGHUP, SIGTERM},
    flag, low_level,
};
use std::{
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

pub struct TerminationSignals {
    received: Arc<AtomicUsize>,
    registrations: Vec<signal_hook::SigId>,
}

impl TerminationSignals {
    pub fn register() -> io::Result<Self> {
        let received = Arc::new(AtomicUsize::new(0));
        let mut signals = Self {
            received,
            registrations: Vec::new(),
        };

        for signal in [SIGHUP, SIGTERM] {
            let registration =
                flag::register_usize(signal, Arc::clone(&signals.received), signal as usize)?;
            signals.registrations.push(registration);
        }

        Ok(signals)
    }

    pub fn received(&self) -> bool {
        self.received_signal().is_some()
    }

    pub fn received_signal(&self) -> Option<i32> {
        let signal = self.received.load(Ordering::SeqCst);
        (signal != 0).then_some(signal as i32)
    }
}

impl Drop for TerminationSignals {
    fn drop(&mut self) {
        for registration in self.registrations.drain(..) {
            low_level::unregister(registration);
        }
    }
}
