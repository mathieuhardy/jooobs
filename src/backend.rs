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
    fn schedule(&mut self, job: Job) -> Result<(), ApiError>;

    /// Run a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be run.
    /// * `messages_channel` - Channel used to send messages to the queue.
    async fn run(
        &mut self,
        id: &Uuid,
        messages_channel: SharedMessageChannel,
    ) -> Result<(), ApiError>;

    /// Get the status of a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be fetched.
    ///
    /// # Returns
    /// The status of the job.
    fn status(&self, id: &Uuid) -> Result<Status, ApiError>;

    /// Set the status of a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be modified.
    fn set_status(&mut self, id: &Uuid, status: Status) -> Result<(), ApiError>;

    /// Get the result of a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be fetched.
    ///
    /// # Returns
    /// The result of the job as list of bytes.
    fn result(&self, id: &Uuid) -> Result<&[u8], ApiError>;

    /// Set the number of steps for a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be modified.
    /// * `steps` - Number of steps to set.
    fn set_steps(&mut self, id: &Uuid, steps: u64) -> Result<(), ApiError>;

    /// Set the current step for a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be modified.
    /// * `step` - Current step to set.
    fn set_step(&mut self, id: &Uuid, step: u64) -> Result<(), ApiError>;

    /// Get the progression of a job.
    ///
    /// # Arguments:
    /// * `id` - Job identifier to be fetched.
    ///
    /// # Returns
    /// The progression of the job as `Progression`.
    fn progression(&self, id: &Uuid) -> Result<Progression, ApiError>;
}

/// Type used to share the backend instance across threads.
pub type SharedBackend<Routine> = Arc<Mutex<Box<dyn Backend<Routine>>>>;
