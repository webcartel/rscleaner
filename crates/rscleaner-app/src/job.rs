use std::thread::JoinHandle;

use crossbeam_channel::Receiver;
use rscleaner_model::CancelToken;

use crate::EngineError;

/// A running job: a bounded event channel, a cancel flag, and a result.
pub struct Job<E, R> {
    pub events: Receiver<E>,
    pub cancel: CancelToken,
    worker: JoinHandle<Result<R, EngineError>>,
}

impl<E, R> Job<E, R> {
    pub(crate) fn new(
        events: Receiver<E>,
        cancel: CancelToken,
        worker: JoinHandle<Result<R, EngineError>>,
    ) -> Self {
        Self {
            events,
            cancel,
            worker,
        }
    }

    /// Wait for the job to finish.
    pub fn result(self) -> Result<R, EngineError> {
        match self.worker.join() {
            Ok(result) => result,
            Err(_) => Err(EngineError::Panic),
        }
    }
}
