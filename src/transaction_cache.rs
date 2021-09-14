use fs::OpenOptions;
use std::fs;

use std::collections::HashMap;
use tempdir::TempDir;

use crate::type_defs::{Transaction, TransactionId};
use serde::{Deserialize, Serialize};

/// Type which represents a CacheKey identifier.
#[derive(Debug, Default, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
struct CacheKey<const CACHE_LINE_SIZE: u32>(u32);

impl<const CACHE_LINE_SIZE: u32> From<TransactionId> for CacheKey<CACHE_LINE_SIZE> {
    fn from(transaction_id: TransactionId) -> Self {
        CacheKey(transaction_id.0 / CACHE_LINE_SIZE)
    }
}

/// Type which represents a CacheLine
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct CacheLine {
    loaded: bool,
    transactions: HashMap<TransactionId, Transaction>,
}

/// Type which abstracts a cache of transactions it behaves exactly as a HashMap
/// with the benefit that it tracks how many records are stored in memory and
/// it goes beyond a certain threshold define by the CACHE_SIZE_LIMIT generic it
/// serializes the caches into files on disk.
#[derive(Debug)]
pub struct TransactionCache<const CACHE_SIZE_LIMIT: u64, const CACHE_LINE_SIZE: u32> {
    cache: HashMap<CacheKey<CACHE_LINE_SIZE>, CacheLine>,
    cache_size: u64,
    cache_size_limit: u64,
    cache_dir: TempDir,
}

impl<const CACHE_SIZE_LIMIT: u64, const CACHE_LINE_SIZE: u32>
    TransactionCache<CACHE_SIZE_LIMIT, CACHE_LINE_SIZE>
{
    pub fn new() -> Result<Self, String> {
        let tmp_dir = TempDir::new("transaction_cache")
            .map_err(|err| format!("Could not create cache dir because of: {}", err))?;
        Ok(TransactionCache {
            cache: HashMap::new(),
            cache_size: 0,
            cache_size_limit: CACHE_SIZE_LIMIT,
            cache_dir: tmp_dir,
        })
    }

    pub fn get(&mut self, transaction_id: &TransactionId) -> Option<&Transaction> {
        let cache_key = CacheKey::from(*transaction_id);
        let mut cache_line = self
            .cache
            .entry(cache_key)
            .or_insert_with(CacheLine::default);

        self.cache_size += Self::load_cache(&self.cache_dir, cache_key, &mut cache_line);
        cache_line.transactions.get(transaction_id)
    }

    pub fn contains_key(&mut self, transaction_id: &TransactionId) -> bool {
        let cache_key = CacheKey::from(*transaction_id);
        let mut cache_line = self
            .cache
            .entry(cache_key)
            .or_insert_with(CacheLine::default);

        self.cache_size += Self::load_cache(&self.cache_dir, cache_key, &mut cache_line);
        cache_line.transactions.contains_key(transaction_id)
    }

    pub fn remove(&mut self, transaction_id: &TransactionId) -> Option<Transaction> {
        let cache_key = CacheKey::from(*transaction_id);
        let cache_line = self
            .cache
            .entry(cache_key)
            .or_insert_with(CacheLine::default);
        self.cache_size += Self::load_cache(&self.cache_dir, cache_key, cache_line);
        cache_line.transactions.remove(transaction_id)
    }

    fn load_cache(
        cache_dir: &TempDir,
        cache_key: CacheKey<CACHE_LINE_SIZE>,
        cache_line: &mut CacheLine,
    ) -> u64 {
        let cache_file_name = Self::cache_path(cache_dir.path().to_str().unwrap(), &cache_key);
        let cache_file = std::path::Path::new(&cache_file_name);
        let mut num_loaded = 0;
        if !cache_line.loaded && cache_file.exists() {
            let file = OpenOptions::new().read(true).open(cache_file).unwrap();

            let stored_cache_lines: HashMap<TransactionId, Transaction> =
                serde_json::from_reader(file).unwrap();
            num_loaded = stored_cache_lines.len();
            cache_line.transactions.extend(stored_cache_lines);
            cache_line.loaded = true;
        }
        num_loaded as u64
    }

    fn store_cache(&mut self) {
        if self.cache_size > self.cache_size_limit {
            for (cache_key, cache_line) in self.cache.iter() {
                Self::store_cache_line(
                    self.cache_dir.path().to_str().unwrap(),
                    cache_key,
                    cache_line,
                );
            }
            self.cache.clear();
            self.cache_size = 0;
        }
    }

    fn cache_path(cache_save_prefix: &str, cache_key: &CacheKey<CACHE_LINE_SIZE>) -> String {
        format!("{}/{}", cache_save_prefix, cache_key.0)
    }

    fn store_cache_line(
        cache_save_prefix: &str,
        cache_key: &CacheKey<CACHE_LINE_SIZE>,
        cache_line: &CacheLine,
    ) {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(Self::cache_path(cache_save_prefix, cache_key))
            .unwrap();
        serde_json::to_writer(file, &cache_line.transactions).unwrap();
    }

    pub fn insert(
        &mut self,
        transaction_id: TransactionId,
        transaction: Transaction,
    ) -> Option<Transaction> {
        let val = self
            .cache
            .entry(CacheKey::from(transaction_id))
            .or_insert_with(CacheLine::default)
            .transactions
            .insert(transaction_id, transaction);
        self.cache_size += 1;
        self.store_cache();
        val
    }
}
