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
impl<RoutineType, Context> Backend<RoutineType, Context> for MemoryBackend
where
    RoutineType: Routine<Context> + Sync,
    for<'async_trait> Context: Send + 'async_trait,
{
    fn schedule(&mut self, job: Job) -> Result<(), ApiError> {
        self.jobs.insert(job.id(), job);

        Ok(())
    }

    async fn run(
        &mut self,
        id: &Uuid,
        context: Option<Shared<Context>>,
        messages_channel: SharedMessageChannel,
    ) -> Result<(), ApiError> {
        if let Some(job) = self.jobs.get_mut(id) {
            job.run::<RoutineType, Context>(messages_channel, context)
                .await?;

            Ok(())
        } else {
            Err(api_err!(Error::JobNotFound(id.to_owned())))
        }
    }

    fn status(&self, id: &Uuid) -> Result<Status, ApiError> {
        Ok(self
            .jobs
            .get(id)
            .ok_or(api_err!(Error::JobNotFound(id.to_owned())))?
            .status())
    }

    fn set_status(&mut self, id: &Uuid, status: Status) -> Result<(), ApiError> {
        if let Some(job) = self.jobs.get_mut(id) {
            job.set_status(status)?;

            Ok(())
        } else {
            Err(api_err!(Error::JobNotFound(id.to_owned())))
        }
    }

    fn result(&self, id: &Uuid) -> Result<&[u8], ApiError> {
        Ok(self
            .jobs
            .get(id)
            .ok_or(api_err!(Error::JobNotFound(id.to_owned())))?
            .result())
    }

    fn set_steps(&mut self, id: &Uuid, steps: u64) -> Result<Progression, ApiError> {
        if let Some(job) = self.jobs.get_mut(id) {
            job.set_steps(steps)
        } else {
            Err(api_err!(Error::JobNotFound(id.to_owned())))
        }
    }

    fn set_step(&mut self, id: &Uuid, step: u64) -> Result<Progression, ApiError> {
        if let Some(job) = self.jobs.get_mut(id) {
            job.set_step(step)
        } else {
            Err(api_err!(Error::JobNotFound(id.to_owned())))
        }
    }

    fn progression(&self, id: &Uuid) -> Result<Progression, ApiError> {
        Ok(self
            .jobs
            .get(id)
            .ok_or(api_err!(Error::JobNotFound(id.to_owned())))?
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
