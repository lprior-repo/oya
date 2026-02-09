//! QueueActor - Manages a single queue of ready beads.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tracing::info;

use crate::actors::supervisor::GenericSupervisableActor;
use crate::scheduler::QueueType;

#[derive(Clone, Default)]
pub struct QueueActorDef;

pub struct QueueState {
    pub queue_id: String,
    pub queue_type: QueueType,
    fifo: VecDeque<String>,
    priority: BinaryHeap<PriorityItem>,
}

#[derive(Debug, Clone)]
pub enum QueueMessage {
    Enqueue {
        bead_id: String,
        priority: Option<u32>,
    },
    Dequeue,
    Peek,
    Length,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueResponse {
    Ack,
    Bead(Option<String>),
    Length(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PriorityItem {
    priority: u32,
    bead_id: String,
}

impl Ord for PriorityItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| self.bead_id.cmp(&other.bead_id))
    }
}

impl PartialOrd for PriorityItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl QueueState {
    fn enqueue(&mut self, bead_id: String, priority: Option<u32>) {
        match self.queue_type {
            QueueType::Priority => {
                let item = PriorityItem {
                    priority: priority.map_or(0, |value| value),
                    bead_id,
                };
                self.priority.push(item);
            }
            _ => {
                self.fifo.push_back(bead_id);
            }
        }
    }

    fn dequeue(&mut self) -> Option<String> {
        match self.queue_type {
            QueueType::Priority => self.priority.pop().map(|item| item.bead_id),
            QueueType::LIFO => self.fifo.pop_back(),
            _ => self.fifo.pop_front(),
        }
    }

    fn peek(&self) -> Option<String> {
        match self.queue_type {
            QueueType::Priority => self.priority.peek().map(|item| item.bead_id.clone()),
            QueueType::LIFO => self.fifo.back().cloned(),
            _ => self.fifo.front().cloned(),
        }
    }

    fn len(&self) -> usize {
        match self.queue_type {
            QueueType::Priority => self.priority.len(),
            _ => self.fifo.len(),
        }
    }
}

impl Actor for QueueActorDef {
    type Msg = QueueMessage;
    type State = QueueState;
    type Arguments = (String, QueueType);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!(queue_id = %args.0, "QueueActor starting");
        Ok(QueueState {
            queue_id: args.0,
            queue_type: args.1,
            fifo: VecDeque::new(),
            priority: BinaryHeap::new(),
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            QueueMessage::Enqueue { bead_id, priority } => {
                state.enqueue(bead_id, priority);
            }
            QueueMessage::Dequeue => {
                let _ = state.dequeue();
            }
            QueueMessage::Peek => {
                let _ = state.peek();
            }
            QueueMessage::Length => {
                let _ = state.len();
            }
        }
        Ok(())
    }
}

impl GenericSupervisableActor for QueueActorDef {
    fn default_args() -> Self::Arguments {
        ("default-queue".to_string(), QueueType::FIFO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fifo_state() -> QueueState {
        QueueState {
            queue_id: String::from("fifo"),
            queue_type: QueueType::FIFO,
            fifo: VecDeque::new(),
            priority: BinaryHeap::new(),
        }
    }

    fn priority_state() -> QueueState {
        QueueState {
            queue_id: String::from("priority"),
            queue_type: QueueType::Priority,
            fifo: VecDeque::new(),
            priority: BinaryHeap::new(),
        }
    }

    #[test]
    fn test_fifo_enqueue_dequeue_order() {
        let mut state = fifo_state();
        state.enqueue(String::from("a"), None);
        state.enqueue(String::from("b"), None);
        state.enqueue(String::from("c"), None);

        assert_eq!(state.dequeue(), Some(String::from("a")));
        assert_eq!(state.dequeue(), Some(String::from("b")));
        assert_eq!(state.dequeue(), Some(String::from("c")));
        assert_eq!(state.dequeue(), None);
    }

    #[test]
    fn test_fifo_peek_and_len() {
        let mut state = fifo_state();
        assert_eq!(state.peek(), None);
        assert_eq!(state.len(), 0);

        state.enqueue(String::from("alpha"), None);
        state.enqueue(String::from("beta"), None);

        assert_eq!(state.peek(), Some(String::from("alpha")));
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn test_priority_dequeue_order() {
        let mut state = priority_state();
        state.enqueue(String::from("low"), Some(1));
        state.enqueue(String::from("high"), Some(100));
        state.enqueue(String::from("mid"), Some(10));

        assert_eq!(state.dequeue(), Some(String::from("high")));
        assert_eq!(state.dequeue(), Some(String::from("mid")));
        assert_eq!(state.dequeue(), Some(String::from("low")));
    }

    #[test]
    fn test_priority_peek_and_len() {
        let mut state = priority_state();
        state.enqueue(String::from("x"), Some(2));
        state.enqueue(String::from("y"), Some(4));

        assert_eq!(state.peek(), Some(String::from("y")));
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn test_lifo_mode_works() {
        let mut state = fifo_state();
        state.queue_type = QueueType::LIFO;
        state.enqueue(String::from("one"), None);
        state.enqueue(String::from("two"), None);
        state.enqueue(String::from("three"), None);

        assert_eq!(state.dequeue(), Some(String::from("three")));
        assert_eq!(state.dequeue(), Some(String::from("two")));
    }

    #[tokio::test]
    async fn test_concurrency_like_enqueues_preserve_count() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let state = Arc::new(Mutex::new(fifo_state()));
        let mut handles = Vec::new();

        for index in 0..32 {
            let state_clone = Arc::clone(&state);
            handles.push(tokio::spawn(async move {
                let mut guard = state_clone.lock().await;
                guard.enqueue(format!("bead-{index}"), None);
            }));
        }

        for handle in handles {
            let join_result = handle.await;
            assert!(join_result.is_ok());
        }

        let guard = state.lock().await;
        assert_eq!(guard.len(), 32);
    }

    #[tokio::test]
    async fn test_concurrency_like_dequeues_drain_all_items() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let mut initial = fifo_state();
        for index in 0..16 {
            initial.enqueue(format!("bead-{index}"), None);
        }

        let state = Arc::new(Mutex::new(initial));
        let mut handles = Vec::new();

        for _ in 0..16 {
            let state_clone = Arc::clone(&state);
            handles.push(tokio::spawn(async move {
                let mut guard = state_clone.lock().await;
                let _ = guard.dequeue();
            }));
        }

        for handle in handles {
            let join_result = handle.await;
            assert!(join_result.is_ok());
        }

        let guard = state.lock().await;
        assert_eq!(guard.len(), 0);
    }
}
