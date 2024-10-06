use async_trait::async_trait;
use lazy_static::lazy_static;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::prelude::*;

lazy_static! {
    /// Random data used to generate UUID V1 values.
    static ref GROUP_ID: [u8; 6] = rand::thread_rng().gen::<[u8; 6]>();
}

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

/// Structure used to store the progression steps of the job.
#[derive(Debug, Deserialize, Serialize)]
pub struct Progression {
    /// Current step.
    pub step: u64,

    /// Number of steps.
    pub steps: u64,
}

/// Structure used to store timestamps and result of the job.
#[derive(PartialEq, Deserialize, Serialize)]
pub struct Payload {
    /// Timestamps for every step of the job.
    pub timestamps: Timestamps,

    /// Result of the job.
    pub result: Vec<u8>,
}

/// Timestamps of every steps of the lifecycle of a job.
#[derive(PartialEq, Deserialize, Serialize)]
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
pub trait Routine<Context>: for<'a> Deserialize<'a> + Serialize + Send {
    /// Function that is called when the job is processed.
    ///
    /// # Arguments
    /// * `job_id` - ID of the job this routine belongs to.
    /// * `messages_channel` - Channel to be used to send back messages to the job queue.
    ///
    /// # Returns
    /// A list of bytes: the result of the job to be stored.
    ///
    /// # Errors
    /// One of `Error` enum.
    async fn call(
        &self,
        job_id: Uuid,
        messages_channel: SharedMessageChannel,
        context: Option<Shared<Context>>,
    ) -> Result<Vec<u8>, Error>;
}

/// Description of a job.
#[derive(PartialEq, Deserialize, Serialize)]
pub struct Job {
    /// Unique identifier of the job.
    id: Uuid,

    /// The routine called when running.
    routine: String,

    /// Status of the job.
    status: Status,

    /// Payload of the job where the result will be stored.
    payload: Payload,

    /// Number of steps of the job.
    steps: u64,

    /// Current step (progression).
    step: u64,
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
    pub fn new<Context>(routine: impl Routine<Context>) -> Result<Self, ApiError> {
        Ok(Self {
            id: Uuid::now_v1(&GROUP_ID),
            routine: serde_json::to_string(&routine).map_err(|e| api_err!(e.into()))?,
            status: Status::NotReady,
            payload: Payload {
                timestamps: Timestamps {
                    enqueued: SystemTime::now(),
                    started: SystemTime::UNIX_EPOCH,
                    finished: SystemTime::UNIX_EPOCH,
                },
                result: vec![],
            },
            steps: 0,
            step: 0,
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
    pub fn set_status(&mut self, status: Status) -> Result<(), ApiError> {
        match status {
            Status::NotReady => {
                return Err(api_err!(Error::InvalidJobStatusTransition((
                    self.status,
                    status
                ))))
            }

            Status::Ready => {
                if self.status != Status::NotReady {
                    return Err(api_err!(Error::InvalidJobStatusTransition((
                        self.status,
                        status
                    ))));
                } else {
                    self.payload.timestamps.enqueued = SystemTime::now();
                }
            }

            Status::Running => {
                if self.status != Status::Ready {
                    return Err(api_err!(Error::InvalidJobStatusTransition((
                        self.status,
                        status
                    ))));
                } else {
                    self.payload.timestamps.started = SystemTime::now();
                }
            }

            Status::Finished => {
                if self.status != Status::Running {
                    return Err(api_err!(Error::InvalidJobStatusTransition((
                        self.status,
                        status
                    ))));
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
    pub fn set_result(&mut self, value: Vec<u8>) -> Result<(), ApiError> {
        self.payload.result = value;

        Ok(())
    }

    /// Set the total steps of the job.
    ///
    /// # Arguments
    /// * `steps` - Number of steps of the job.
    ///
    /// # Returns
    /// Current progression.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn set_steps(&mut self, steps: u64) -> Result<Progression, ApiError> {
        if self.step <= steps {
            self.steps = steps;

            Ok(self.progression())
        } else {
            Err(api_err!(Error::ProgressionOverflow))
        }
    }

    /// Set the current step of the job.
    ///
    /// # Arguments
    /// * `step` - Current step of the job.
    ///
    /// # Returns
    /// Current progression.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn set_step(&mut self, step: u64) -> Result<Progression, ApiError> {
        if step <= self.steps {
            self.step = step;

            Ok(self.progression())
        } else {
            Err(api_err!(Error::ProgressionOverflow))
        }
    }

    /// Get the progression of the job.
    ///
    /// # Returns
    /// The progression of the job.
    pub fn progression(&self) -> Progression {
        Progression {
            step: self.step,
            steps: self.steps,
        }
    }

    /// Call the underlying routine of the job.
    ///
    /// # Arguments
    /// * `messages_channel` - Channel used to send message to the job queue.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub async fn run<T: Routine<Context>, Context>(
        &mut self,
        messages_channel: SharedMessageChannel,
        context: Option<Shared<Context>>,
    ) -> Result<(), ApiError> {
        // Routine information is stored as string so deserialize it
        let routine: T = serde_json::from_str(&self.routine).map_err(|e| api_err!(e.into()))?;

        // Call the routine
        let result = routine.call(self.id(), messages_channel, context).await?;

        // Store the result
        self.set_result(result)
    }
}
