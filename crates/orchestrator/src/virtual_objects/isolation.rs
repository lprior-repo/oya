//! State isolation and locking for virtual objects.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

use super::object::ObjectId;

/// Isolation level for object operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    /// Read committed - can see committed changes.
    ReadCommitted,
    /// Repeatable read - same reads within transaction.
    RepeatableRead,
    /// Serializable - full isolation.
    #[default]
    Serializable,
}

/// A lock on a virtual object.
#[derive(Debug)]
pub struct ObjectLock {
    object_id: ObjectId,
    lock: Arc<RwLock<()>>,
    lock_count: Arc<RwLock<u32>>,
}

impl ObjectLock {
    /// Create a new object lock.
    #[must_use]
    pub fn new(object_id: impl Into<ObjectId>) -> Self {
        Self {
            object_id: object_id.into(),
            lock: Arc::new(RwLock::new(())),
            lock_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Get the object ID.
    #[must_use]
    pub fn object_id(&self) -> &ObjectId {
        &self.object_id
    }

    /// Acquire a read lock.
    pub async fn read(&self) -> ObjectLockGuard {
        let guard = Arc::clone(&self.lock).read_owned().await;
        ObjectLockGuard::Read(guard)
    }

    /// Acquire a write lock.
    pub async fn write(&self) -> ObjectLockGuard {
        let guard = Arc::clone(&self.lock).write_owned().await;
        {
            let mut count = self.lock_count.write().await;
            *count = count.saturating_add(1);
        }
        ObjectLockGuard::Write(guard)
    }

    /// Try to acquire a read lock with timeout.
    pub async fn try_read(&self, timeout: Duration) -> Option<ObjectLockGuard> {
        let lock = Arc::clone(&self.lock);
        tokio::time::timeout(timeout, async move { lock.read_owned().await })
            .await
            .ok()
            .map(ObjectLockGuard::Read)
    }

    /// Try to acquire a write lock with timeout.
    pub async fn try_write(&self, timeout: Duration) -> Option<ObjectLockGuard> {
        let lock = Arc::clone(&self.lock);
        let lock_count = Arc::clone(&self.lock_count);

        tokio::time::timeout(timeout, async move {
            let guard = lock.write_owned().await;
            {
                let mut count = lock_count.write().await;
                *count = count.saturating_add(1);
            }
            guard
        })
        .await
        .ok()
        .map(ObjectLockGuard::Write)
    }

    /// Get the number of times the lock has been acquired for writing.
    pub async fn write_count(&self) -> u32 {
        *self.lock_count.read().await
    }
}

/// Guard for an object lock.
pub enum ObjectLockGuard {
    /// Read guard.
    Read(OwnedRwLockReadGuard<()>),
    /// Write guard.
    Write(OwnedRwLockWriteGuard<()>),
}

impl ObjectLockGuard {
    /// Check if this is a read lock.
    #[must_use]
    pub fn is_read(&self) -> bool {
        matches!(self, Self::Read(_))
    }

    /// Check if this is a write lock.
    #[must_use]
    pub fn is_write(&self) -> bool {
        matches!(self, Self::Write(_))
    }
}

/// Information about a lock holder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    /// Object being locked
    pub object_id: ObjectId,
    /// Holder identifier
    pub holder_id: String,
    /// When the lock was acquired
    pub acquired_at: DateTime<Utc>,
    /// Lock type (read/write)
    pub lock_type: LockType,
    /// Isolation level
    pub isolation_level: IsolationLevel,
}

/// Type of lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockType {
    /// Read lock (shared).
    Read,
    /// Write lock (exclusive).
    Write,
}

/// Manages locks across multiple objects.
pub struct LockManager {
    locks: Arc<RwLock<HashMap<String, ObjectLock>>>,
    active_locks: Arc<RwLock<Vec<LockInfo>>>,
}

impl LockManager {
    /// Create a new lock manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
            active_locks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get or create a lock for an object.
    pub async fn get_lock(&self, object_id: impl Into<ObjectId>) -> ObjectLock {
        let id = object_id.into();
        let id_str = id.as_str().to_string();

        {
            let locks = self.locks.read().await;
            if let Some(lock) = locks.get(&id_str) {
                return ObjectLock {
                    object_id: lock.object_id.clone(),
                    lock: Arc::clone(&lock.lock),
                    lock_count: Arc::clone(&lock.lock_count),
                };
            }
        }

        // Create new lock
        let lock = ObjectLock::new(id.clone());

        {
            let mut locks = self.locks.write().await;
            locks.insert(
                id_str.clone(),
                ObjectLock {
                    object_id: id,
                    lock: Arc::clone(&lock.lock),
                    lock_count: Arc::clone(&lock.lock_count),
                },
            );
        }

        lock
    }

    /// Acquire a read lock.
    pub async fn acquire_read(
        &self,
        object_id: impl Into<ObjectId>,
        holder_id: impl Into<String>,
        isolation_level: IsolationLevel,
    ) -> ObjectLockGuard {
        let id = object_id.into();
        let holder = holder_id.into();

        let lock = self.get_lock(id.clone()).await;
        let guard = lock.read().await;

        // Record active lock
        {
            let mut active = self.active_locks.write().await;
            active.push(LockInfo {
                object_id: id,
                holder_id: holder,
                acquired_at: Utc::now(),
                lock_type: LockType::Read,
                isolation_level,
            });
        }

        guard
    }

    /// Acquire a write lock.
    pub async fn acquire_write(
        &self,
        object_id: impl Into<ObjectId>,
        holder_id: impl Into<String>,
        isolation_level: IsolationLevel,
    ) -> ObjectLockGuard {
        let id = object_id.into();
        let holder = holder_id.into();

        let lock = self.get_lock(id.clone()).await;
        let guard = lock.write().await;

        // Record active lock
        {
            let mut active = self.active_locks.write().await;
            active.push(LockInfo {
                object_id: id,
                holder_id: holder,
                acquired_at: Utc::now(),
                lock_type: LockType::Write,
                isolation_level,
            });
        }

        guard
    }

    /// Release a lock (called when guard is dropped).
    pub async fn release(&self, object_id: &ObjectId, holder_id: &str) {
        let mut active = self.active_locks.write().await;
        active.retain(|info| !(info.object_id == *object_id && info.holder_id == holder_id));
    }

    /// Get active locks for an object.
    pub async fn get_active_locks(&self, object_id: &ObjectId) -> Vec<LockInfo> {
        let active = self.active_locks.read().await;
        active
            .iter()
            .filter(|info| &info.object_id == object_id)
            .cloned()
            .collect()
    }

    /// Get all active locks.
    pub async fn all_active_locks(&self) -> Vec<LockInfo> {
        let active = self.active_locks.read().await;
        active.clone()
    }

    /// Get the number of managed locks.
    pub async fn lock_count(&self) -> usize {
        let locks = self.locks.read().await;
        locks.len()
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_level_default() {
        let level = IsolationLevel::default();
        assert!(matches!(level, IsolationLevel::Serializable));
    }

    #[test]
    fn test_lock_type_variants() {
        assert!(matches!(LockType::Read, LockType::Read));
        assert!(matches!(LockType::Write, LockType::Write));
    }

    #[tokio::test]
    async fn test_object_lock_read() {
        let lock = ObjectLock::new("obj-1");

        let guard = lock.read().await;
        assert!(guard.is_read());
    }

    #[tokio::test]
    async fn test_object_lock_write() {
        let lock = ObjectLock::new("obj-1");

        let guard = lock.write().await;
        assert!(guard.is_write());
        assert_eq!(lock.write_count().await, 1);
    }

    #[tokio::test]
    async fn test_object_lock_multiple_reads() {
        let lock = ObjectLock::new("obj-1");

        let _guard1 = lock.read().await;
        let _guard2 = lock.read().await;

        // Multiple readers should be allowed
        assert!(_guard1.is_read());
        assert!(_guard2.is_read());
    }

    #[tokio::test]
    async fn test_object_lock_try_read_timeout() {
        let lock = ObjectLock::new("obj-1");

        let result = lock.try_read(Duration::from_millis(100)).await;
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_lock_manager_get_lock() {
        let manager = LockManager::new();

        let lock1 = manager.get_lock("obj-1").await;
        let lock2 = manager.get_lock("obj-1").await;

        // Should return locks for the same object
        assert_eq!(lock1.object_id().as_str(), "obj-1");
        assert_eq!(lock2.object_id().as_str(), "obj-1");
    }

    #[tokio::test]
    async fn test_lock_manager_acquire_read() {
        let manager = LockManager::new();

        let guard = manager
            .acquire_read("obj-1", "holder-1", IsolationLevel::ReadCommitted)
            .await;

        assert!(guard.is_read());

        let active = manager.get_active_locks(&ObjectId::new("obj-1")).await;
        assert_eq!(active.len(), 1);
        assert!(matches!(active[0].lock_type, LockType::Read));
    }

    #[tokio::test]
    async fn test_lock_manager_acquire_write() {
        let manager = LockManager::new();

        let guard = manager
            .acquire_write("obj-1", "holder-1", IsolationLevel::Serializable)
            .await;

        assert!(guard.is_write());

        let active = manager.get_active_locks(&ObjectId::new("obj-1")).await;
        assert_eq!(active.len(), 1);
        assert!(matches!(active[0].lock_type, LockType::Write));
    }

    #[tokio::test]
    async fn test_lock_manager_release() {
        let manager = LockManager::new();

        let _guard = manager
            .acquire_read("obj-1", "holder-1", IsolationLevel::ReadCommitted)
            .await;

        manager.release(&ObjectId::new("obj-1"), "holder-1").await;

        let active = manager.get_active_locks(&ObjectId::new("obj-1")).await;
        assert!(active.is_empty());
    }

    #[tokio::test]
    async fn test_lock_manager_all_active_locks() {
        let manager = LockManager::new();

        let _guard1 = manager
            .acquire_read("obj-1", "holder-1", IsolationLevel::ReadCommitted)
            .await;
        let _guard2 = manager
            .acquire_write("obj-2", "holder-2", IsolationLevel::Serializable)
            .await;

        let all = manager.all_active_locks().await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_lock_manager_lock_count() {
        let manager = LockManager::new();

        let _ = manager.get_lock("obj-1").await;
        let _ = manager.get_lock("obj-2").await;
        let _ = manager.get_lock("obj-3").await;

        assert_eq!(manager.lock_count().await, 3);
    }

    #[test]
    fn test_lock_info_serialization() {
        let info = LockInfo {
            object_id: ObjectId::new("obj-1"),
            holder_id: "holder-1".to_string(),
            acquired_at: Utc::now(),
            lock_type: LockType::Write,
            isolation_level: IsolationLevel::Serializable,
        };

        let json = serde_json::to_string(&info);
        assert!(json.is_ok());
    }
}
