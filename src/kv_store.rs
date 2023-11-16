use hotstuff_rs::state;
use std::collections::{hash_map, hash_set, HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct KVStoreImpl(Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>);

impl KVStoreImpl {
    pub fn new() -> Self {
        Self(Default::default())
    }
}

impl Default for KVStoreImpl {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl state::KVGet for KVStoreImpl {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let map = self.0.read().unwrap();
        map.get(key).cloned()
    }
}

impl state::KVStore for KVStoreImpl {
    type WriteBatch = WriteBatchImpl;

    type Snapshot<'a> = SnapshotImpl;

    fn write(&mut self, wb: Self::WriteBatch) {
        let mut map = self.0.write().unwrap();
        let (inserts, deletes) = wb.consume();
        for (k, v) in inserts {
            map.insert(k, v);
        }
        for k in deletes {
            map.remove(&k);
        }
    }

    fn clear(&mut self) {
        let mut map = self.0.write().unwrap();
        map.clear();
    }

    fn snapshot<'b>(&'b self) -> Self::Snapshot<'_> {
        SnapshotImpl(self.0.clone())
    }
}

pub struct WriteBatchImpl(HashMap<Vec<u8>, Vec<u8>>, HashSet<Vec<u8>>);

impl WriteBatchImpl {
    pub fn consume(
        self,
    ) -> (
        hash_map::IntoIter<Vec<u8>, Vec<u8>>,
        hash_set::IntoIter<Vec<u8>>,
    ) {
        (self.0.into_iter(), self.1.into_iter())
    }
}

impl state::WriteBatch for WriteBatchImpl {
    fn new() -> Self {
        Self(HashMap::new(), HashSet::new())
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.0.insert(key.into(), value.into());
    }

    fn delete(&mut self, key: &[u8]) {
        self.1.insert(key.into());
    }
}

pub struct SnapshotImpl(Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>);

impl state::KVGet for SnapshotImpl {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.read().unwrap().get(key).cloned()
    }
}
