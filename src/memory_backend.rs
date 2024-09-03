use async_trait::async_trait;
use std::collections::BTreeMap;

use crate::prelude::*;

/// A default backend implementation that stores everything in memory.
#[derive(Default)]
pub struct MemoryBackend {
    /// List of jobs stored and sorted by date added.
    jobs: BTreeMap<Uuid, Job>,
}

#[async_trait]
impl<RoutineType> Backend<RoutineType> for MemoryBackend
where
    RoutineType: AsyncRoutine + Sync,
{
    fn schedule(&mut self, job: Job) -> Result<(), Error> {
        self.jobs.insert(job.id(), job);

        Ok(())
    }

    async fn run(&mut self, id: Uuid) -> Result<(), Error> {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.run::<RoutineType>().await?;

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
