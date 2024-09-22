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
    async fn run(&mut self, id: &Uuid) -> Result<(), Error>;

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
}

/// Type used to share the backend instance across threads.
pub type SharedBackend<Routine> = Arc<Mutex<Box<dyn Backend<Routine>>>>;
