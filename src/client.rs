use crate::transaction_cache::TransactionCache;
use crate::type_defs::{Amount, ClientId};
use crate::type_defs::{Transaction, TransactionId};
use csv::Writer;
use std::io::Write;

/// Type that abstracts a Client.
/// It keeps track of all the transactions which reference a certain client and
/// all the disputed transactions.
#[derive(Debug)]
pub struct Client<const CACHE_SIZE_LIMIT: u64, const CACHE_LINE_SIZE: u32> {
    client_id: ClientId,
    available: Amount,
    held: Amount,
    total: Amount,
    locked: bool,
    processed_transactions: TransactionCache<CACHE_SIZE_LIMIT, CACHE_LINE_SIZE>,
    disputed: TransactionCache<CACHE_SIZE_LIMIT, CACHE_LINE_SIZE>,
}

impl<const CACHE_SIZE_LIMIT: u64, const CACHE_LINE_SIZE: u32>
    Client<CACHE_SIZE_LIMIT, CACHE_LINE_SIZE>
{
    pub fn new(client_id: ClientId) -> Result<Self, String> {
        Ok(Self::new_with_cache(
            client_id,
            TransactionCache::new()?,
            TransactionCache::new()?,
        ))
    }

    pub fn new_with_cache(
        client_id: ClientId,
        processed_transactions: TransactionCache<CACHE_SIZE_LIMIT, CACHE_LINE_SIZE>,
        disputed: TransactionCache<CACHE_SIZE_LIMIT, CACHE_LINE_SIZE>,
    ) -> Self {
        Client {
            client_id,
            available: Amount::new(),
            held: Amount::new(),
            total: Amount::new(),
            locked: false,
            processed_transactions,
            disputed,
        }
    }

    pub fn can_process(&self) -> Result<(), String> {
        if self.locked {
            return Err("Account locked".to_owned());
        }
        Ok(())
    }
    pub fn deposit(&mut self, transaction: Transaction) -> Result<(), String> {
        self.can_process()?;
        if let Transaction::Deposit(_, transaction_id, amount) = transaction {
            if self.processed_transactions.contains_key(&transaction_id) {
                return Err("Transaction already processed".to_owned());
            }
            self.available += amount;
            self.total += amount;
            self.processed_transactions
                .insert(transaction_id, transaction);
            return Ok(());
        }
        Err("Wrong transaction type, expected deposit".to_owned())
    }

    pub fn withdraw(&mut self, transaction: Transaction) -> Result<(), String> {
        self.can_process()?;

        if let Transaction::Withdrawal(_, transaction_id, amount) = transaction {
            if self.processed_transactions.contains_key(&transaction_id) {
                return Err("Transaction already processed".to_owned());
            }

            if amount <= self.available {
                self.available -= amount;
                self.total -= amount;
                self.processed_transactions
                    .insert(transaction_id, transaction);
                return Ok(());
            }
            return Err("Insufficient funds".to_owned());
        }

        Err("Wrong transaction type, expected withdraw".to_owned())
    }

    pub fn dispute(&mut self, disputed_transaction_id: &TransactionId) -> Result<(), String> {
        if self.disputed.contains_key(disputed_transaction_id) {
            return Err("Transaction already processed".to_owned());
        }

        let disputed_transaction = self
            .processed_transactions
            .get(disputed_transaction_id)
            .ok_or("Could not find disputed transaction")?;
        if let Transaction::Deposit(_, transaction_id, amount) = disputed_transaction {
            self.available -= *amount;
            self.held += *amount;
            self.disputed.insert(*transaction_id, *disputed_transaction);
            return Ok(());
        }

        Err("Wrong transaction type".to_owned())
    }

    pub fn resolve(&mut self, disputed_transaction_id: &TransactionId) -> Result<(), String> {
        self.can_process()?;

        let disputed_transaction = self
            .disputed
            .remove(disputed_transaction_id)
            .ok_or("Could not find disputed transaction")?;
        if let Transaction::Deposit(_, _, amount) = disputed_transaction {
            self.available += amount;
            self.held -= amount;
            return Ok(());
        }
        Err("Wrong transaction type, expected resolve".to_owned())
    }

    pub fn chargeback(&mut self, disputed_transaction_id: &TransactionId) -> Result<(), String> {
        self.can_process()?;

        let disputed_transaction = self
            .disputed
            .remove(disputed_transaction_id)
            .ok_or("Could not find disputed transaction")?;
        if let Transaction::Deposit(_, _, amount) = disputed_transaction {
            self.locked = true;
            self.total -= amount;
            self.held -= amount;
            return Ok(());
        }

        Err("Wrong transaction type, expected resolve".to_owned())
    }

    pub fn serialize<W: Write>(self, writer: &mut Writer<W>) -> Result<(), String> {
        writer
            .serialize((
                self.client_id.0,
                self.available.to_string(),
                self.held.to_string(),
                self.total.to_string(),
                self.locked,
            ))
            .map_err(|err| format!("Could not serialize client because of: {}", err))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn client_id(&self) -> ClientId {
        self.client_id
    }

    #[allow(dead_code)]
    pub fn available(&self) -> Amount {
        self.available
    }

    #[allow(dead_code)]
    pub fn held(&self) -> Amount {
        self.held
    }

    #[allow(dead_code)]
    pub fn total(&self) -> Amount {
        self.total
    }

    #[allow(dead_code)]
    pub fn locked(&self) -> bool {
        self.locked
    }
}
