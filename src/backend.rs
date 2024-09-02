use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::prelude::*;

/// Backend trait that defines the behavior of the backend that is responsible for storing the job
/// and their results.
pub trait Backend: Send + Sync {
    fn schedule(&mut self, job: Job) -> Result<(), Error>;
    fn run(&mut self, id: Uuid) -> Result<(), Error>;
    fn status(&self, id: Uuid) -> Result<Status, Error>;
    fn set_status(&mut self, id: Uuid, status: Status) -> Result<(), Error>;
}

/// Type used to share the backend instance across threads.
pub type SharedBackend = Arc<Mutex<Box<dyn Backend>>>;

/// A default backend implementation that stores everything in memory.
#[derive(Default)]
pub struct MemoryBackend {
    /// List of jobs stored and sorted by date added.
    jobs: BTreeMap<Uuid, Job>,
}

impl Backend for MemoryBackend {
    fn schedule(&mut self, mut job: Job) -> Result<(), Error> {
        job.set_status(Status::Ready)?;

        self.jobs.insert(job.id(), job);

        Ok(())
    }

    fn run(&mut self, id: Uuid) -> Result<(), Error> {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.run();

            Ok(())
        } else {
            Err(Error::JobNotFound(id))
        }
    }

    fn status(&self, id: Uuid) -> Result<Status, Error> {
        Ok(self.jobs.get(&id).ok_or(Error::JobNotFound(id))?.status())
    }

    fn set_status(&mut self, id: Uuid, status: Status) -> Result<(), Error> {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.set_status(status)?;

            Ok(())
        } else {
            Err(Error::JobNotFound(id))
        }
    }
}

impl MemoryBackend {
    /// Creates a new instance of the memory backend.
    ///
    /// # Returns
    /// A instance of ̀̀ MemoryBackend`.
    pub fn new() -> Self {
        Self::default()
    }
}
