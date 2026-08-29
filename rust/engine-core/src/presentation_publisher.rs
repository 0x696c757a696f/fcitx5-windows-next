#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use fcitx5_windows_common_core::VerifiedPipeClient;

use crate::{protocol, PresentationPublicationAction, PresentationPublicationQueue};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(25);
const WRITE_TIMEOUT: Duration = Duration::from_millis(25);

struct Shared {
    queue: Mutex<PresentationPublicationQueue>,
    wake: Condvar,
}

pub struct PresentationPublisher {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

impl PresentationPublisher {
    pub fn new(pipe_name: OsString, ui_executable: PathBuf) -> Option<Self> {
        let shared = Arc::new(Shared {
            queue: Mutex::new(PresentationPublicationQueue::new()),
            wake: Condvar::new(),
        });
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("fcitx5-presentation-publisher".to_owned())
            .spawn(move || run(worker_shared, pipe_name, ui_executable))
            .ok()?;
        Some(Self {
            shared,
            worker: Some(worker),
        })
    }

    pub fn publish_frame(&self, frame: &[u8]) -> bool {
        let Some(response) = crate::decode_presentation_frame(frame) else {
            return false;
        };
        let mut queue = self.shared.queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.publish(response);
        self.shared.wake.notify_one();
        true
    }
}

impl Drop for PresentationPublisher {
    fn drop(&mut self) {
        {
            let mut queue = self.shared.queue.lock().unwrap_or_else(|e| e.into_inner());
            queue.stop();
        }
        self.shared.wake.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run(shared: Arc<Shared>, pipe_name: OsString, ui_executable: PathBuf) {
    let mut client = None;
    loop {
        let response = {
            let mut queue = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                match queue.next_action() {
                    PresentationPublicationAction::Stop => return,
                    PresentationPublicationAction::Deliver(response) => break response.clone(),
                    PresentationPublicationAction::Wait => {
                        queue = shared.wake.wait(queue).unwrap_or_else(|e| e.into_inner());
                    }
                    PresentationPublicationAction::DisconnectAndRetryAfter(_) => unreachable!(),
                }
            }
        };

        if client.is_none() {
            client = VerifiedPipeClient::connect_exact(
                OsStr::new(&pipe_name),
                &ui_executable,
                CONNECT_TIMEOUT,
            );
        }
        let delivered = client.as_mut().is_some_and(|client| {
            protocol::encode_key_response(&response)
                .is_some_and(|frame| client.write_all(&frame, WRITE_TIMEOUT))
        });

        if delivered {
            let mut queue = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
            queue.acknowledge_delivery(&response);
        } else {
            client = None;
            let queue = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
            if matches!(queue.next_action(), PresentationPublicationAction::Stop) {
                return;
            }
            let (_queue, result) = shared
                .wake
                .wait_timeout(queue, PresentationPublicationQueue::delivery_failed_delay())
                .unwrap_or_else(|e| e.into_inner());
            if result.timed_out() {
                continue;
            }
        }
    }
}
