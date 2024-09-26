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
    RoutineType: Routine + Sync,
{
    fn schedule(&mut self, job: Job) -> Result<(), Error> {
        self.jobs.insert(job.id(), job);

        Ok(())
    }

    async fn run(&mut self, id: &Uuid, notifications: SharedMessageChannel) -> Result<(), Error> {
        if let Some(job) = self.jobs.get_mut(id) {
            job.run::<RoutineType>(notifications).await?;

            Ok(())
        } else {
            Err(Error::JobNotFound(id.to_owned()))
        }
    }

    fn status(&self, id: &Uuid) -> Result<Status, Error> {
        Ok(self
            .jobs
            .get(id)
            .ok_or(Error::JobNotFound(id.to_owned()))?
            .status())
    }

    fn set_status(&mut self, id: &Uuid, status: Status) -> Result<(), Error> {
        if let Some(job) = self.jobs.get_mut(id) {
            job.set_status(status)?;

            Ok(())
        } else {
            Err(Error::JobNotFound(id.to_owned()))
        }
    }

    fn result(&self, id: &Uuid) -> Result<&[u8], Error> {
        Ok(self
            .jobs
            .get(id)
            .ok_or(Error::JobNotFound(id.to_owned()))?
            .result())
    }

    fn set_steps(&mut self, id: &Uuid, steps: u64) -> Result<(), Error> {
        if let Some(job) = self.jobs.get_mut(id) {
            job.set_steps(steps)?;

            Ok(())
        } else {
            Err(Error::JobNotFound(id.to_owned()))
        }
    }

    fn set_step(&mut self, id: &Uuid, step: u64) -> Result<(), Error> {
        if let Some(job) = self.jobs.get_mut(id) {
            job.set_step(step)?;

            Ok(())
        } else {
            Err(Error::JobNotFound(id.to_owned()))
        }
    }

    fn progression(&self, id: &Uuid) -> Result<Progression, Error> {
        Ok(self
            .jobs
            .get(id)
            .ok_or(Error::JobNotFound(id.to_owned()))?
            .progression())
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
