use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use crate::job_queue::{Message, Notification};

/// Type used to share some instance across threads.
pub type Shared<T> = Arc<Mutex<T>>;

/// Type used to share the error handler across threads.
pub type SharedNotificationHandler<Context> = Arc<dyn Fn(Notification<Context>) + Send + Sync>;

/// Type used to share the message channel.
pub type SharedMessageChannel = Arc<Mutex<Sender<Message>>>;

/// Type used to share the runtime instance across threads.
pub type SharedRuntime = Arc<Mutex<Runtime>>;
