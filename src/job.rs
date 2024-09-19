use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::prelude::*;

/// List of statuses of a job.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
pub enum Status {
    /// Job is not ready and not scheduled.
    #[default]
    NotReady,

    /// Job is not running yet but ready to be started.
    Ready,

    /// Job is currently running.
    Running,

    /// Job is finished (on success or error).
    Finished,
}

/// Structure used to store timestamps and result of the job.
#[derive(Deserialize, Serialize)]
pub struct Payload {
    /// Timestamps for every step of the job.
    pub timestamps: Timestamps,

    /// Result of the job.
    pub result: Vec<u8>,
}

/// Timestamps of every steps of the lifecycle of a job.
#[derive(Deserialize, Serialize)]
pub struct Timestamps {
    /// Timestamp at which the job is enqueued.
    pub enqueued: SystemTime,

    /// Timestamp at which the job is started.
    pub started: SystemTime,

    /// Timestamp at which the job is finished.
    pub finished: SystemTime,
}

/// Trait that must be derived for the list of possible routines handled by the jobs.
#[async_trait]
pub trait Routine: for<'a> Deserialize<'a> + Serialize + Send {
    async fn call(&self) -> Result<Vec<u8>, Error>;
}

/// Description of a job.
#[derive(Deserialize, Serialize)]
pub struct Job {
    /// Unique identifier of the job.
    id: Uuid,

    /// The routine called when running.
    routine: String,

    /// Status of the job.
    status: Status,

    /// Payload of the job where the result will be stored.
    payload: Payload,
}

impl Job {
    /// Creates a new job given a routine to be executed.
    ///
    /// # Arguments
    /// * `routine` - Routine to be called.
    ///
    /// # Returns
    /// An `Job` instance.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn new(routine: impl Routine) -> Result<Self, Error> {
        Ok(Self {
            id: Uuid::now_v1(&[1, 2, 3, 4, 5, 6]),
            routine: serde_json::to_string(&routine)?,
            status: Status::NotReady,
            payload: Payload {
                timestamps: Timestamps {
                    enqueued: SystemTime::now(),
                    started: SystemTime::UNIX_EPOCH,
                    finished: SystemTime::UNIX_EPOCH,
                },
                result: vec![],
            },
        })
    }

    /// Get the unique identifier of the job.
    ///
    /// # Returns
    /// The ID of the job.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the status of the job.
    ///
    /// # Returns
    /// The current status of the job.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Set the status of the job.
    ///
    /// # Arguments
    /// * ̀`status` - Value to be set.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn set_status(&mut self, status: Status) -> Result<(), Error> {
        match status {
            Status::NotReady => {
                return Err(Error::InvalidJobStatusTransition((self.status, status)))
            }

            Status::Ready => {
                if self.status != Status::NotReady {
                    return Err(Error::InvalidJobStatusTransition((self.status, status)));
                } else {
                    self.payload.timestamps.enqueued = SystemTime::now();
                }
            }

            Status::Running => {
                if self.status != Status::Ready {
                    return Err(Error::InvalidJobStatusTransition((self.status, status)));
                } else {
                    self.payload.timestamps.started = SystemTime::now();
                }
            }

            Status::Finished => {
                if self.status != Status::Running {
                    return Err(Error::InvalidJobStatusTransition((self.status, status)));
                } else {
                    self.payload.timestamps.finished = SystemTime::now();
                }
            }
        }

        self.status = status;

        Ok(())
    }

    /// Get the result of the job.
    ///
    /// # Returns
    /// The result of the job.
    pub fn result(&self) -> &[u8] {
        &self.payload.result
    }

    /// Set the result of the job that will be stored as `serde_json::Value`.
    ///
    /// # Arguments
    /// * `value` - Serializable value to be stored.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn set_result(&mut self, value: Vec<u8>) -> Result<(), Error> {
        self.payload.result = value;

        Ok(())
    }

    /// Call the underlying routine of the job.
    pub async fn run<T: Routine>(&mut self) -> Result<(), Error> {
        let routine: T = serde_json::from_str(&self.routine)?;

        let result = routine.call().await?;

        self.set_result(result)
    }
}
