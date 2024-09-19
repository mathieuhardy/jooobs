use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::prelude::*;

/// Backend trait that defines the behavior of the backend that is responsible for storing the job
/// and their results.
#[async_trait]
pub trait Backend<Routine>: Send {
    // TODO: document
    fn schedule(&mut self, job: Job) -> Result<(), Error>;
    // TODO: document
    async fn run(&mut self, id: &Uuid) -> Result<(), Error>;
    // TODO: document
    fn status(&self, id: &Uuid) -> Result<Status, Error>;
    // TODO: document
    fn set_status(&mut self, id: &Uuid, status: Status) -> Result<(), Error>;
    // TODO: document
    fn result(&self, id: &Uuid) -> Result<&[u8], Error>;
}

/// Type used to share the backend instance across threads.
pub type SharedBackend<Routine> = Arc<Mutex<Box<dyn Backend<Routine>>>>;
