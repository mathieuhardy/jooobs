pub mod job;
pub mod job_queue;
pub mod prelude;

// TODO: Result for routine returns
// TODO: Job ID
// TODO: Result storage
// TODO: Get status of a job

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::prelude::*;

    macro_rules! toggle {
        () => {{
            let value = Arc::new(Mutex::new(false));
            let cloned = value.clone();
            (value, cloned)
        }};
    }

    macro_rules! set_toggle {
        ($ident: expr) => {{
            *$ident.lock().unwrap() = true;
        }};
    }

    macro_rules! check_toggle {
        ($ident: expr) => {{
            assert!(*$ident.lock().unwrap());
        }};
    }

    #[tokio::test]
    async fn nominal() {
        let mut jq = JobQueue::new(1, 8).unwrap();
        let (t, t2) = toggle!();

        jq.start().unwrap();
        assert_eq!(jq.state(), State::Running);

        let routine = Box::new(move || {
            set_toggle!(t2);
        });

        jq.enqueue(Job::new(routine)).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        check_toggle!(t);

        jq.stop().await.unwrap();
        jq.join().await.unwrap();
        assert_eq!(jq.state(), State::Stopped);
    }
}
