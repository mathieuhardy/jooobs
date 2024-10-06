pub mod backend;
pub mod error;
pub mod job;
pub mod job_queue;
pub mod job_queue_builder;
pub mod memory_backend;
pub mod prelude;
pub mod types;

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Mutex;
    use tokio::runtime::Runtime;

    use crate::prelude::*;

    static FLAG: Mutex<bool> = Mutex::new(false);

    struct Context {
        name: String,
    }

    fn notification_handler(notification: Notification) {
        match notification {
            Notification::Error(e) => println!("ERR: {e}"),

            Notification::Progression(id, progression) => {
                println!("PROGRESSION({id}): {progression:#?}")
            }

            Notification::Status(id, status) => {
                println!("STATUS({id}): {status:#?}")
            }
        }
    }

    fn reset_flag() {
        set_flag(SetFlagArgs { value: false });
    }

    fn check_flag() {
        assert!(*FLAG.lock().unwrap());
    }

    fn set_flag(args: SetFlagArgs) {
        *FLAG.lock().unwrap() = args.value;
    }

    #[derive(Clone, Serialize, Deserialize)]
    struct SetFlagArgs {
        value: bool,
    }

    #[derive(Serialize, Deserialize)]
    enum Routines {
        SetFlag(SetFlagArgs),
    }

    #[async_trait]
    impl Routine<Context> for Routines {
        async fn call(
            &self,
            job_id: Uuid,
            messages_channel: SharedMessageChannel,
            context: Option<Shared<Context>>,
        ) -> Result<Vec<u8>, Error> {
            if let Some(context) = context {
                assert_eq!(&context.lock().unwrap().name, "UNIT_TESTING")
            }

            match self {
                Self::SetFlag(args) => {
                    let messages_channel = messages_channel.lock().unwrap();

                    set_flag(args.clone());

                    messages_channel
                        .send(Message::Command(Cmd::SetSteps(job_id, 2)))
                        .unwrap();

                    let json = serde_json::json!({
                        "result": "SET_FLAG_OK",
                    });

                    messages_channel
                        .send(Message::Command(Cmd::SetStep(job_id, 1)))
                        .unwrap();

                    let bytes = json.to_string().into_bytes();

                    messages_channel
                        .send(Message::Command(Cmd::SetStep(job_id, 2)))
                        .unwrap();

                    Ok(bytes)
                }
            }
        }
    }

    #[test]
    fn nominal() {
        let mut jq = JobQueueBuilder::<Routines, Context>::new(1)
            .unwrap()
            .notification_handler(notification_handler)
            .context(Context {
                name: "UNIT_TESTING".to_string(),
            })
            .build();

        reset_flag();

        // Start queue
        jq.start().unwrap();
        assert_eq!(jq.state(), State::Running);

        Runtime::new().unwrap().block_on(async {
            // Create the job and push it
            let routine = Routines::SetFlag(SetFlagArgs { value: true });
            let job = Job::new(routine).unwrap();
            let job_id = job.id();

            jq.enqueue(job).unwrap();

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Verify that job has been processed
            check_flag();
            let bytes = jq.job_result(&job_id).await.unwrap();
            let result: Value = serde_json::from_slice(&bytes).unwrap();
            let status = jq.job_status(&job_id).await.unwrap();
            let progression = jq.job_progression(&job_id).await.unwrap();
            assert_eq!(result["result"], "SET_FLAG_OK");
            assert_eq!(status, Status::Finished);
            assert_eq!(progression.step, 2);
            assert_eq!(progression.steps, 2);

            // Stop the job queue
            jq.stop().unwrap();
        });

        jq.join().unwrap();
    }

    mod errors {
        use super::*;

        #[test]
        fn not_startable() {
            let mut jq = JobQueueBuilder::<Routines, Context>::new(1)
                .unwrap()
                .notification_handler(notification_handler)
                .build();

            Runtime::new().unwrap().block_on(async {
                jq.start().unwrap();
                assert!(jq.start().is_err());
            });
        }

        #[test]
        fn not_joinable() {
            let jq = JobQueueBuilder::<Routines, Context>::new(1)
                .unwrap()
                .notification_handler(notification_handler)
                .build();

            assert!(jq.join().is_err());

            let mut jq = JobQueueBuilder::<Routines, Context>::new(1)
                .unwrap()
                .notification_handler(notification_handler)
                .build();

            Runtime::new().unwrap().block_on(async {
                jq.start().unwrap();
            });

            assert!(jq.join().is_err());
        }

        #[test]
        fn not_stoppable() {
            let mut jq = JobQueueBuilder::<Routines, Context>::new(1)
                .unwrap()
                .notification_handler(notification_handler)
                .build();

            Runtime::new().unwrap().block_on(async {
                assert!(jq.stop().is_err());

                jq.start().unwrap();
                jq.stop().unwrap();
                assert!(jq.stop().is_err());
            });
        }
    }
}
