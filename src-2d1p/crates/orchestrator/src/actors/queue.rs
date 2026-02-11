//! `QueueActor` - Manages a single queue of ready beads.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, VecDeque};

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
    tenant_queues: HashMap<String, VecDeque<String>>,
    tenant_rotation: VecDeque<String>,
}

#[derive(Debug, Clone)]
pub enum QueueMessage {
    Enqueue {
        bead_id: String,
        priority: Option<u32>,
        tenant_id: Option<String>,
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
    fn enqueue(&mut self, bead_id: String, priority: Option<u32>, tenant_id: Option<String>) {
        match self.queue_type {
            QueueType::Priority => {
                let item = PriorityItem {
                    priority: priority.map_or(0, |value| value),
                    bead_id,
                };
                self.priority.push(item);
            }
            QueueType::RoundRobin => {
                let tenant = tenant_id.unwrap_or_else(|| String::from("default"));
                let is_new_tenant = !self.tenant_queues.contains_key(&tenant);
                if is_new_tenant {
                    self.tenant_rotation.push_back(tenant.clone());
                }
                self.tenant_queues
                    .entry(tenant)
                    .or_default()
                    .push_back(bead_id);
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
            QueueType::RoundRobin => self.dequeue_round_robin(),
            _ => self.fifo.pop_front(),
        }
    }

    fn peek(&self) -> Option<String> {
        match self.queue_type {
            QueueType::Priority => self.priority.peek().map(|item| item.bead_id.clone()),
            QueueType::LIFO => self.fifo.back().cloned(),
            QueueType::RoundRobin => self.tenant_rotation.iter().find_map(|tenant| {
                self.tenant_queues
                    .get(tenant)
                    .and_then(|queue| queue.front().cloned())
            }),
            _ => self.fifo.front().cloned(),
        }
    }

    fn len(&self) -> usize {
        match self.queue_type {
            QueueType::Priority => self.priority.len(),
            QueueType::RoundRobin => self.tenant_queues.values().map(VecDeque::len).sum(),
            _ => self.fifo.len(),
        }
    }

    fn dequeue_round_robin(&mut self) -> Option<String> {
        let tenant_count = self.tenant_rotation.len();
        for _ in 0..tenant_count {
            let tenant = self.tenant_rotation.pop_front()?;
            let maybe_bead = self
                .tenant_queues
                .get_mut(&tenant)
                .and_then(VecDeque::pop_front);

            match maybe_bead {
                Some(bead) => {
                    let tenant_has_more = self
                        .tenant_queues
                        .get(&tenant)
                        .is_some_and(|queue| !queue.is_empty());
                    if tenant_has_more {
                        self.tenant_rotation.push_back(tenant);
                    } else {
                        let _ = self.tenant_queues.remove(&tenant);
                    }
                    return Some(bead);
                }
                None => {
                    let _ = self.tenant_queues.remove(&tenant);
                }
            }
        }
        None
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
            tenant_queues: HashMap::new(),
            tenant_rotation: VecDeque::new(),
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            QueueMessage::Enqueue {
                bead_id,
                priority,
                tenant_id,
            } => {
                state.enqueue(bead_id, priority, tenant_id);
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
            tenant_queues: HashMap::new(),
            tenant_rotation: VecDeque::new(),
        }
    }

    fn priority_state() -> QueueState {
        QueueState {
            queue_id: String::from("priority"),
            queue_type: QueueType::Priority,
            fifo: VecDeque::new(),
            priority: BinaryHeap::new(),
            tenant_queues: HashMap::new(),
            tenant_rotation: VecDeque::new(),
        }
    }

    fn round_robin_state() -> QueueState {
        QueueState {
            queue_id: String::from("round-robin"),
            queue_type: QueueType::RoundRobin,
            fifo: VecDeque::new(),
            priority: BinaryHeap::new(),
            tenant_queues: HashMap::new(),
            tenant_rotation: VecDeque::new(),
        }
    }

    #[test]
    fn test_fifo_enqueue_dequeue_order() {
        let mut state = fifo_state();
        state.enqueue(String::from("a"), None, None);
        state.enqueue(String::from("b"), None, None);
        state.enqueue(String::from("c"), None, None);

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

        state.enqueue(String::from("alpha"), None, None);
        state.enqueue(String::from("beta"), None, None);

        assert_eq!(state.peek(), Some(String::from("alpha")));
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn test_priority_dequeue_order() {
        let mut state = priority_state();
        state.enqueue(String::from("low"), Some(1), None);
        state.enqueue(String::from("high"), Some(100), None);
        state.enqueue(String::from("mid"), Some(10), None);

        assert_eq!(state.dequeue(), Some(String::from("high")));
        assert_eq!(state.dequeue(), Some(String::from("mid")));
        assert_eq!(state.dequeue(), Some(String::from("low")));
    }

    #[test]
    fn test_priority_peek_and_len() {
        let mut state = priority_state();
        state.enqueue(String::from("x"), Some(2), None);
        state.enqueue(String::from("y"), Some(4), None);

        assert_eq!(state.peek(), Some(String::from("y")));
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn test_lifo_mode_works() {
        let mut state = fifo_state();
        state.queue_type = QueueType::LIFO;
        state.enqueue(String::from("one"), None, None);
        state.enqueue(String::from("two"), None, None);
        state.enqueue(String::from("three"), None, None);

        assert_eq!(state.dequeue(), Some(String::from("three")));
        assert_eq!(state.dequeue(), Some(String::from("two")));
    }

    #[test]
    fn test_round_robin_enqueue_auto_creates_tenant_queue() {
        let mut state = round_robin_state();
        state.enqueue(String::from("bead-a"), None, Some(String::from("tenant-a")));
        state.enqueue(String::from("bead-b"), None, Some(String::from("tenant-b")));

        assert_eq!(state.len(), 2);
        assert_eq!(state.peek(), Some(String::from("bead-a")));
    }

    #[test]
    fn test_round_robin_dequeue_fair_rotation() {
        let mut state = round_robin_state();

        state.enqueue(
            String::from("tenant-1-a"),
            None,
            Some(String::from("tenant-1")),
        );
        state.enqueue(
            String::from("tenant-2-a"),
            None,
            Some(String::from("tenant-2")),
        );
        state.enqueue(
            String::from("tenant-3-a"),
            None,
            Some(String::from("tenant-3")),
        );
        state.enqueue(
            String::from("tenant-1-b"),
            None,
            Some(String::from("tenant-1")),
        );
        state.enqueue(
            String::from("tenant-2-b"),
            None,
            Some(String::from("tenant-2")),
        );
        state.enqueue(
            String::from("tenant-3-b"),
            None,
            Some(String::from("tenant-3")),
        );

        assert_eq!(state.dequeue(), Some(String::from("tenant-1-a")));
        assert_eq!(state.dequeue(), Some(String::from("tenant-2-a")));
        assert_eq!(state.dequeue(), Some(String::from("tenant-3-a")));
        assert_eq!(state.dequeue(), Some(String::from("tenant-1-b")));
        assert_eq!(state.dequeue(), Some(String::from("tenant-2-b")));
        assert_eq!(state.dequeue(), Some(String::from("tenant-3-b")));
        assert_eq!(state.dequeue(), None);
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
                guard.enqueue(format!("bead-{index}"), None, None);
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
            initial.enqueue(format!("bead-{index}"), None, None);
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
