# Usage

**Create a handler to catch the notifications**

```rust
fn notification_handler(notif: Notification) {
    match notif { ... }
}
```

**Create a context structure (even if don't need any).

```rust
struct Context {
    ...
}
```

**Create a routine with its arguments**

```rust
#[derive(Clone, Serialize, Deserialize)]
struct MyRoutineArgs {
    first_arg: String,
}

#[derive(Serialize, Deserialize)]
enum Routines {
    MyRoutine(MyRoutineArgs),
}
```

**Implement the mandatory trait to your routine enum**

```rust
#[async_trait]
impl Routine<Context> for Routines {
    async fn call(
        &self,
        job_id: Uuid,
        messages: SharedMessageChannel,
        context: Option<Shared<Context>>,
    ) -> Result<Vec<u8>, Error> {
        if let Some(context) = context {
            let ctx = context.lock().unwrap();
            // ...
        }

        match self {
            Self::MyRoutine(args) => {
                let messages_channel = messages_channel.lock().unwrap();
                let steps_count = 10;

                // Set the number of steps for this routine
                messages_channel
                    .send(Message::Command(Cmd::SetSteps(job_id, steps_count)))
                    .unwrap();

                // Run long process
                for idx in 0..steps_count {
                    // ...

                    messages_channel
                        .send(Message::Command(Cmd::SetStep(job_id, idx)))
                        .unwrap();
                }

                // Build vector of bytes to be returned for storage
                let job_result = [..];

                Ok(job_result)
            }
        }
    }
}
```

**Instantiate the job queue**

```rust
let thread_pool_size = 8;

let mut jq = JobQueueBuilder::<Routines>::new()
    .unwrap()
    .notification_handler(notification_handler)
    .context(Context{})
    .build();

jq.start().unwrap();

...

jq.stop().unwrap();
jq.join().unwrap();
```

**Push a job into the queue**

```rust
// Create the routine
let routine = Routines::MyRoutine(MyRoutineArgs { first_arg: 'Hello World'.to_string() });

// Create a job from this routine
let job = Job::new(routine).unwrap();

// Push it
jq.enqueue(job).unwrap();
```

**Get information of a job**

```rust
// Get result (as bytes)
let bytes = jq.job_result(&job_id).await.unwrap();

// Get status of the job
let status = jq.job_status(&job_id).await.unwrap();

// Get progression of the job (if provided by the job itself)
let progression = jq.job_progression(&job_id).await.unwrap();
```

# TODO

- Expiration feature
- Retry/clean at startup
- Think about pipeline or sub job parallelism
- Improve unit tests
