use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::prelude::*;

/// Backend trait that defines the behavior of the backend that is responsible for storing the job
/// and their results.
#[async_trait]
pub trait Backend<Routine>: Send {
    /// Schedule a job to be processed.
    ///
    /// # Arguments:
    /// * `job` - Job structure to be processed.
    fn schedule(&mut self, job: Job) -> Result<(), Error>;

    /// Run a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be run.
    async fn run(&mut self, id: &Uuid, notifications: SharedMessageChannel) -> Result<(), Error>;

    /// Get the status of a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be fetched.
    ///
    /// # Returns
    /// The status of the job.
    fn status(&self, id: &Uuid) -> Result<Status, Error>;

    /// Set the status of a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be modified.
    fn set_status(&mut self, id: &Uuid, status: Status) -> Result<(), Error>;

    /// Get the result of a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be fetched.
    ///
    /// # Returns
    /// The result of the job as list of bytes.
    fn result(&self, id: &Uuid) -> Result<&[u8], Error>;

    /// Set the number of steps for a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be modified.
    /// * `steps` - Number of steps to set.
    fn set_steps(&mut self, id: &Uuid, steps: u64) -> Result<(), Error>;

    /// Set the current step for a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be modified.
    /// * `step` - Current step to set.
    fn set_step(&mut self, id: &Uuid, step: u64) -> Result<(), Error>;

    /// Get the progression of a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be fetched.
    ///
    /// # Returns
    /// The progression of the job as `Progression`.
    fn progression(&self, id: &Uuid) -> Result<Progression, Error>;
}

/// Type used to share the backend instance across threads.
pub type SharedBackend<Routine> = Arc<Mutex<Box<dyn Backend<Routine>>>>;
