pub mod job_queue;

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::job_queue::*;

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

        let job = Box::new(move || {
            set_toggle!(t2);
        });

        jq.enqueue(Job { callback: job }).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(00)).await;

        check_toggle!(t);
    }
}
