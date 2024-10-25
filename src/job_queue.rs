use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tokio::runtime::Builder;
use tokio::sync::Mutex as AsyncMutex;

use crate::memory_backend::*;
use crate::prelude::*;

/// Type of messages that can be sent to the job queue.
#[derive(PartialEq)]
pub enum Message {
    /// Command message that change the state of the queue.
    Command(Cmd),

    /// Job message used to push a new job to be processed.
    Job(Job),
}

/// Commands handled by the thread of the job queue.
#[derive(PartialEq)]
pub enum Cmd {
    /// Set current step for a job.
    SetStep(Uuid, u64),

    /// Set number of steps for a job.
    SetSteps(Uuid, u64),

    /// Stop the job queue.
    Stop,
}

/// Type of notifications that can be sent from the job queue.
#[derive(Debug)]
pub enum Notification {
    /// Error notification.
    Error(Error),

    /// Update of the progression of a job.
    Progression(Uuid, Progression),

    /// Update of the status of a job.
    Status(Uuid, Status),
}

/// States of the tread running the job queue.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum State {
    /// Idle (i.e. waiting to be started).
    #[default]
    Idle,

    /// Running and processing jobs.
    Running,

    /// Going to stop.
    Stopping,
}

/// Structure of a job queue.
pub struct JobQueue<RoutineType, Context> {
    /// State of the job queue.
    state: State,

    /// Channel used to send messages to the thread of the job queue.
    tx: SharedMessageChannel,

    /// Channel used to receive messages from the thread of the job queue.
    rx: Shared<Receiver<Message>>,

    /// Join handle used to wait the thread of the job queue.
    join_handle: Option<JoinHandle<()>>,

    /// Backend used to store the list of jobs with their results.
    backend: SharedBackend<RoutineType, Context>,

    /// Tokio runtime instance with dedicated thread pool.
    runtime: SharedRuntime,

    /// Notification handler function.
    notification_handler: SharedNotificationHandler,

    /// Context to be passed to every routine.
    context: Option<Shared<Context>>,
}

impl<RoutineType, Context> JobQueue<RoutineType, Context>
where
    RoutineType: Routine<Context> + Sync + 'static,
    Context: Send + 'static,
{
    /// Creates a new job queue.
    ///
    /// # Arguments
    /// * `thread_pool_size` - Number of thread to allocate in the internal thread pool.
    ///
    /// # Returns
    /// An instance of `JobQueue`.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn new(thread_pool_size: Option<usize>) -> Result<Self, ApiError> {
        // This Tokio runtime will carry the threads for each job.
        let mut builder = Builder::new_multi_thread();

        builder.enable_io();

        if let Some(thread_pool_size) = thread_pool_size {
            if thread_pool_size == 0 {
                return Err(api_err!(Error::InvalidThreadPoolSize));
            }

            builder.worker_threads(thread_pool_size);
        }

        let runtime = builder.build().map_err(|e| api_err!(e.into()))?;

        // Create the channel for communicating with the thread of the queue.
        let (tx, rx) = std::sync::mpsc::channel();

        Ok(Self {
            state: State::default(),
            tx: Arc::new(Mutex::new(tx)),
            rx: Arc::new(Mutex::new(rx)),
            join_handle: None,
            backend: Arc::new(AsyncMutex::new(Box::new(MemoryBackend::new()))),
            runtime: Arc::new(Mutex::new(runtime)),
            notification_handler: Arc::new(|_| {}),
            context: None,
        })
    }

    /// Gets the state of the queue.
    ///
    /// # Returns
    /// A value of the enum `State`.
    pub fn state(&self) -> State {
        self.state
    }

    /// Sets the backend used by the queue to store jobs and their results.
    ///
    /// # Arguments:
    /// * `backend` - Backend instance that will replace the current one.
    pub fn set_backend(&mut self, backend: impl Backend<RoutineType, Context> + 'static) {
        self.backend = Arc::new(AsyncMutex::new(Box::new(backend)));
    }

    /// Sets the notification handler used by the queue to notify client.
    ///
    /// # Arguments:
    /// * `handler` - Handler instance that will replace the current one.
    pub fn set_notification_handler(
        &mut self,
        handler: impl Fn(Notification) + Send + Sync + 'static,
    ) {
        self.notification_handler = Arc::new(handler);
    }

    /// Sets the context to be passed to every routine.
    ///
    /// # Arguments:
    /// * `context` - Context instance to set.
    pub fn set_context(&mut self, context: Context) {
        self.context = Some(Arc::new(Mutex::new(context)));
    }

    /// Starts the job queue with async support.
    ///
    /// # Errors
    /// One of `Error` enum.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn start(&mut self) -> Result<(), ApiError> {
        self.try_starting()?;

        // Clone ressources to be used by the thread
        let backend = self.backend.clone();
        let runtime = self.runtime.clone();
        let rx = self.rx.clone();
        let notification_handler = self.notification_handler.clone();
        let messages_channel = self.tx.clone();
        let context = self.context.clone();

        let handle = std::thread::spawn(move || {
            let rx = match rx.lock() {
                Ok(rx) => rx,
                Err(e) => {
                    notification_handler(Notification::Error(Error::CannotAccessReceiver(
                        e.to_string(),
                    )));

                    return;
                }
            };

            while let Ok(msg) = rx.recv() {
                // Special case used to stop the thread.
                if msg == Message::Command(Cmd::Stop) {
                    break;
                }

                // Process the message received: job or command.
                JobQueue::process_message(
                    backend.clone(),
                    runtime.clone(),
                    notification_handler.clone(),
                    messages_channel.clone(),
                    context.clone(),
                    msg,
                );
            }
        });

        self.join_handle = Some(handle);
        self.state = State::Running;

        Ok(())
    }

    /// Tries to join the job queue waiting it to finish.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn join(self) -> Result<(), ApiError> {
        self.try_joining()?;

        if let Some(handle) = self.join_handle {
            if handle.join().is_err() {
                return Err(api_err!(Error::CannotJoinThread));
            }

            // TODO
            //self.runtime
            //.lock()
            //.unwrap()
            //.shutdown_timeout(std::time::Duration::from_millis(100));

            Ok(())
        } else {
            Err(api_err!(Error::MissingJoinHandle))
        }
    }

    /// Send a stop command to the queue.
    /// There's no garantee that it will be processed but we'll do our best.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn stop(&mut self) -> Result<(), ApiError> {
        self.try_stopping()?;

        self.state = State::Stopping;

        self.tx
            .lock()
            .map_err(|e| api_err!(Error::CannotAccessSender(e.to_string())))?
            .send(Message::Command(Cmd::Stop))
            .map_err(|e| api_err!(e.into()))
    }

    /// Push a new job to be processed in the queue.
    ///
    /// # Arguments
    /// * `job` - Job to be enqueued.
    ///
    /// # Returns
    /// The unique ID of the job.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn enqueue(&self, job: Job) -> Result<Uuid, ApiError> {
        let job_id = job.id();

        self.tx
            .lock()
            .map_err(|e| api_err!(Error::CannotAccessSender(e.to_string())))?
            .send(Message::Job(job))
            .map_err(Into::<Error>::into)?;

        Ok(job_id)
    }

    /// Get the status of a job.
    ///
    /// # Arguments
    /// * `id` - ID of the job to be inspected.
    ///
    /// # Returns
    /// The status of the job.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub async fn job_status(&self, id: &Uuid) -> Result<Status, ApiError> {
        self.backend.lock().await.status(id)
    }

    /// Get the result of a job.
    ///
    /// # Arguments
    /// * `id` - ID of the job to be inspected.
    ///
    /// # Returns
    /// The result of the job as vector of bytes.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub async fn job_result(&self, id: &Uuid) -> Result<Vec<u8>, ApiError> {
        Ok(self.backend.lock().await.result(id)?.to_vec())
    }

    /// Get the progression of a job.
    ///
    /// # Arguments
    /// * `id` - ID of the job to be inspected.
    ///
    /// # Returns
    /// The progression of the job.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub async fn job_progression(&self, id: &Uuid) -> Result<Progression, ApiError> {
        self.backend.lock().await.progression(id)
    }

    /// Checks if the current state allows to start the queue.
    ///
    /// # Errors
    /// One of `Error` enum.
    fn try_starting(&self) -> Result<(), ApiError> {
        match self.state {
            State::Running => Err(api_err!(Error::AlreadyRunning)),
            State::Stopping => Err(api_err!(Error::Stopped)),
            _ => Ok(()),
        }
    }

    /// Checks if the current state allows to join the queue.
    ///
    /// # Errors
    /// One of `Error` enum.
    fn try_joining(&self) -> Result<(), ApiError> {
        match self.state {
            State::Idle => Err(api_err!(Error::NotStarted)),
            State::Running => Err(api_err!(Error::NotStopping)),
            _ => Ok(()),
        }
    }

    /// Checks if the current state allows to stop the queue.
    ///
    /// # Errors
    /// One of `Error` enum.
    fn try_stopping(&self) -> Result<(), ApiError> {
        match self.state {
            State::Idle => Err(api_err!(Error::NotStarted)),
            State::Stopping => Err(api_err!(Error::Stopped)),
            _ => Ok(()),
        }
    }

    /// Processes a message (can be a command or job).
    ///
    /// # Arguments
    /// * `backend` - Backend instance used to process the jobs.
    /// * `runtime` - Runtime carrying the thread pool.
    /// * `notification_handler` - Handler for notifications.
    /// * `messages_channel` - Channel used to communicate with the queue thread.
    /// * `context` - Context used by the jobs.
    /// * `msg` - Message to be processed.
    fn process_message(
        backend: SharedBackend<RoutineType, Context>,
        runtime: SharedRuntime,
        notification_handler: SharedNotificationHandler,
        messages_channel: Shared<Sender<Message>>,
        context: Option<Shared<Context>>,
        msg: Message,
    ) {
        match msg {
            Message::Job(job) => {
                let _ = JobQueue::process_job(
                    backend,
                    runtime,
                    notification_handler.clone(),
                    messages_channel.clone(),
                    context,
                    job,
                )
                .map_err(|e| notification_handler(Notification::Error(*e)));
            }

            Message::Command(cmd) => {
                let _ =
                    JobQueue::process_command(backend, runtime, notification_handler.clone(), cmd)
                        .map_err(|e| notification_handler(Notification::Error(*e)));
            }
        }
    }

    /// Processes a command.
    ///
    /// # Arguments
    /// * `backend` - Backend instance used to process the jobs.
    /// * `runtime` - Runtime carrying the thread pool.
    /// * `notification_handler` - Handler for notifications.
    /// * `cmd` - Command to be processed.
    ///
    /// # Errors
    /// One of `Error` enum.
    fn process_command(
        backend: SharedBackend<RoutineType, Context>,
        runtime: SharedRuntime,
        notification_handler: SharedNotificationHandler,
        cmd: Cmd,
    ) -> Result<(), ApiError> {
        let runtime = runtime
            .lock()
            .map_err(|e| Error::CannotAccessRuntime(e.to_string()))?;

        runtime.block_on(async {
            let mut backend = backend.lock().await;

            match cmd {
                Cmd::SetSteps(job_id, steps) => {
                    if let Ok(p) = backend
                        .set_steps(&job_id, steps)
                        .map_err(|e| notification_handler(Notification::Error(*e)))
                    {
                        notification_handler(Notification::Progression(job_id, p));
                    }
                }

                Cmd::SetStep(job_id, step) => {
                    if let Ok(p) = backend
                        .set_step(&job_id, step)
                        .map_err(|e| notification_handler(Notification::Error(*e)))
                    {
                        notification_handler(Notification::Progression(job_id, p));
                    }
                }

                _ => (),
            }
        });

        Ok(())
    }

    /// Processes a job.
    ///
    /// # Arguments
    /// * `backend` - Backend instance used to process the jobs.
    /// * `runtime` - Runtime carrying the thread pool.
    /// * `notification_handler` - Handler for notifications.
    /// * `messages_channel` - Channel used to communicate with the queue thread.
    /// * `context` - Context used by the jobs.
    /// * `job` - Job to be processed.
    ///
    /// # Errors
    /// One of `Error` enum.
    fn process_job(
        backend: SharedBackend<RoutineType, Context>,
        runtime: SharedRuntime,
        notification_handler: SharedNotificationHandler,
        messages_channel: Shared<Sender<Message>>,
        context: Option<Shared<Context>>,
        job: Job,
    ) -> Result<(), ApiError> {
        let job_id = job.id();

        let runtime = runtime
            .lock()
            .map_err(|e| Error::CannotAccessRuntime(e.to_string()))?;

        runtime.block_on(async {
            let mut backend = backend.lock().await;

            // Push the job in the backend (to be stored)
            if backend
                .schedule(job)
                .map_err(|e| notification_handler(Notification::Error(*e)))
                .is_err()
            {
                return;
            }

            // Set its status to ready (can be processed)
            let _ = backend
                .set_status(&job_id, Status::Ready)
                .map_err(|e| notification_handler(Notification::Error(*e)));
        });

        runtime.spawn(async move {
            let mut backend = backend.lock().await;

            // Seet status of the job to `Status::Running`
            if backend
                .set_status(&job_id, Status::Running)
                .map_err(|e| notification_handler(Notification::Error(*e)))
                .is_err()
            {
                return;
            }

            notification_handler(Notification::Status(job_id, Status::Running));

            // Call the routine of the job
            let result_status = match backend
                .run(&job_id, context, messages_channel)
                .await
                .map_err(|e| notification_handler(Notification::Error(*e)))
            {
                Ok(_) => ResultStatus::Success,
                _ => ResultStatus::Error,
            };

            // Set status of the job to `Status::Finished`
            let status = Status::Finished(result_status);

            if backend
                .set_status(&job_id, status)
                .map_err(|e| notification_handler(Notification::Error(*e)))
                .is_err()
            {
                return;
            }

            notification_handler(Notification::Status(job_id, status));
        });

        Ok(())
    }
}
