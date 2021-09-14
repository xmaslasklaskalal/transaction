use std::collections::HashMap;
use std::io;

use crate::client::Client;
use crate::type_defs::{ClientId, Transaction, TransactionRecord};

/// Assume we have at least 2GiB available to store transactions in memory.
pub const CACHE_SIZE_LIMIT: u64 = 2 * 1024 * 1024 * 1024;
/// Each cache line could have 4 MiB.
pub const CACHE_SIZE_LINE: u32 = 4 * 1024 * 1024;

/// Type that abstracts an transaction processor, it is the entry point for processing
/// any transaction.
pub struct TransactionProcessor<const CACHE_SIZE_LIMIT: u64, const CACHE_LINE_SIZE: u32> {
    clients: HashMap<ClientId, Client<CACHE_SIZE_LIMIT, CACHE_LINE_SIZE>>,
}

impl<const CACHE_SIZE_LIMIT: u64, const CACHE_LINE_SIZE: u32>
    TransactionProcessor<CACHE_SIZE_LIMIT, CACHE_LINE_SIZE>
{
    pub fn new() -> Self {
        TransactionProcessor {
            clients: HashMap::new(),
        }
    }

    /// Processes a transaction and reports in case any erros is encountered.
    pub fn process_transaction(&mut self, record: TransactionRecord) -> Result<(), String> {
        let transaction = Transaction::from_record(record)?;
        match transaction {
            Transaction::Deposit(client_id, _, _) => self
                .clients
                .entry(client_id)
                .or_insert(Client::new(client_id)?)
                .deposit(transaction),
            Transaction::Withdrawal(client_id, _, _) => self
                .clients
                .entry(client_id)
                .or_insert(Client::new(client_id)?)
                .withdraw(transaction),

            Transaction::Dispute(client_id, transaction_id) => self
                .clients
                .entry(client_id)
                .or_insert(Client::new(client_id)?)
                .dispute(&transaction_id),

            Transaction::Resolve(client_id, transaction_id) => self
                .clients
                .entry(client_id)
                .or_insert(Client::new(client_id)?)
                .resolve(&transaction_id),
            Transaction::ChargeBack(client_id, transaction_id) => self
                .clients
                .entry(client_id)
                .or_insert(Client::new(client_id)?)
                .chargeback(&transaction_id),
            Transaction::Unknown => Err("Transaction::Unknown".to_owned()),
        }
    }

    /// Serializes the balance acounts for all the clients.
    pub fn serialize(self) -> Result<(), String> {
        let mut wtr = csv::Writer::from_writer(io::stdout());
        wtr.write_record(&["client_id", "available", "held", "total", "locked"])
            .map_err(|err| format!("Could not serialize header because of: {}", err))?;

        for client in self.clients {
            client.1.serialize(&mut wtr)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type_defs::{Amount, ClientId};

    // Test deposit transactions in a loop
    #[test]
    fn test_deposit_loop() {
        let mut processor = TransactionProcessor::<CACHE_SIZE_LIMIT, CACHE_SIZE_LINE>::new();
        let num_iterations = 1024;
        let expected_amount = num_iterations.to_string();
        for i in 0..num_iterations {
            let transaction = TransactionRecord {
                transaction_type: "deposit".to_owned(),
                client: 1,
                tx: i,
                amount: Some("1".to_owned()),
            };
            assert_eq!(processor.process_transaction(transaction), Ok(()));
        }
        assert_eq!(processor.clients.len(), 1);
        for client in processor.clients.into_values() {
            assert_eq!(client.client_id(), ClientId(1));
            assert_eq!(
                client.total(),
                Amount::from_str(expected_amount.clone()).unwrap()
            );
            assert_eq!(
                client.available(),
                Amount::from_str(expected_amount.clone()).unwrap()
            );
            assert_eq!(client.locked(), false);
            assert_eq!(client.held(), Amount::from_str("0.0".to_owned()).unwrap());
        }
    }

    // Test deposit follow by the same amount of withdraws.
    #[test]

    fn test_deposit_withdraw_loop() {
        let mut processor = TransactionProcessor::<CACHE_SIZE_LIMIT, CACHE_SIZE_LINE>::new();
        let num_iterations = 8 * 1024;
        for i in 0..num_iterations {
            let transaction = TransactionRecord {
                transaction_type: "deposit".to_owned(),
                client: 1,
                tx: i * 2,
                amount: Some("1".to_owned()),
            };

            assert_eq!(processor.process_transaction(transaction), Ok(()));

            let transaction = TransactionRecord {
                transaction_type: "withdrawal".to_owned(),
                client: 1,
                tx: i * 2 + 1,
                amount: Some("1".to_owned()),
            };
            assert_eq!(processor.process_transaction(transaction), Ok(()));
        }

        assert_eq!(processor.clients.len(), 1);
        for client in processor.clients.into_values() {
            assert_eq!(client.client_id(), ClientId(1));
            assert_eq!(client.total(), Amount::from_str("0.0".to_owned()).unwrap());
            assert_eq!(
                client.available(),
                Amount::from_str("0.0".to_owned()).unwrap()
            );
            assert_eq!(client.locked(), false);
            assert_eq!(client.held(), Amount::from_str("0.0".to_owned()).unwrap());
        }
    }

    // Test duplicate transaction do nothing.
    #[test]

    fn test_duplicate_transactions_do_nothing() {
        let mut processor = TransactionProcessor::<CACHE_SIZE_LIMIT, CACHE_SIZE_LINE>::new();
        let num_iterations = 8 * 1024;
        for i in 0..num_iterations {
            let transaction = TransactionRecord {
                transaction_type: "deposit".to_owned(),
                client: 1,
                tx: i * 2,
                amount: Some("1".to_owned()),
            };

            assert_eq!(processor.process_transaction(transaction.clone()), Ok(()));
            assert!(processor.process_transaction(transaction).is_err());

            let transaction = TransactionRecord {
                transaction_type: "withdrawal".to_owned(),
                client: 1,
                tx: i * 2 + 1,
                amount: Some("1".to_owned()),
            };
            assert_eq!(processor.process_transaction(transaction.clone()), Ok(()));
            assert!(processor.process_transaction(transaction).is_err());
        }

        assert_eq!(processor.clients.len(), 1);
        for client in processor.clients.into_values() {
            assert_eq!(client.client_id(), ClientId(1));
            assert_eq!(client.total(), Amount::from_str("0.0".to_owned()).unwrap());
            assert_eq!(
                client.available(),
                Amount::from_str("0.0".to_owned()).unwrap()
            );
            assert_eq!(client.locked(), false);
            assert_eq!(client.held(), Amount::from_str("0.0".to_owned()).unwrap());
        }
    }

    // Test a sequence of dispute, withdraw, resolve and make sure the
    // account balance is correct.
    #[test]
    fn test_deposit_dispute_withdraw_resolve_withdraw() {
        let mut processor = TransactionProcessor::<CACHE_SIZE_LIMIT, CACHE_SIZE_LINE>::new();
        let deposit_transaction_id = 8 * 1024;
        let transaction = TransactionRecord {
            transaction_type: "deposit".to_owned(),
            client: 1,
            tx: deposit_transaction_id,
            amount: Some("1".to_owned()),
        };

        assert_eq!(processor.process_transaction(transaction), Ok(()));
        let transaction = TransactionRecord {
            transaction_type: "dispute".to_owned(),
            client: 1,
            tx: deposit_transaction_id,
            amount: None,
        };
        assert_eq!(processor.process_transaction(transaction), Ok(()));

        let transaction = TransactionRecord {
            transaction_type: "withdrawal".to_owned(),
            client: 1,
            tx: deposit_transaction_id + 1,
            amount: Some("1".to_owned()),
        };
        assert!(processor.process_transaction(transaction).is_err());

        let transaction = TransactionRecord {
            transaction_type: "resolve".to_owned(),
            client: 1,
            tx: deposit_transaction_id,
            amount: None,
        };

        assert_eq!(processor.process_transaction(transaction.clone()), Ok(()));

        assert!(processor.process_transaction(transaction).is_err());

        let transaction = TransactionRecord {
            transaction_type: "withdrawal".to_owned(),
            client: 1,
            tx: deposit_transaction_id + 1,
            amount: Some("1".to_owned()),
        };
        assert_eq!(processor.process_transaction(transaction), Ok(()));
        for client in processor.clients.into_values() {
            assert_eq!(client.client_id(), ClientId(1));
            assert_eq!(client.total(), Amount::from_str("0.0".to_owned()).unwrap());
            assert_eq!(
                client.available(),
                Amount::from_str("0.0".to_owned()).unwrap()
            );
            assert_eq!(client.locked(), false);
            assert_eq!(client.held(), Amount::from_str("0.0".to_owned()).unwrap());
        }
    }

    // Test that disputing the same transaction twice or resolving
    // twice do not have any impact.
    #[test]
    fn test_deposit_dispute_twice_resolve_twice() {
        let mut processor = TransactionProcessor::<CACHE_SIZE_LIMIT, CACHE_SIZE_LINE>::new();
        let deposit_transaction_id = 8 * 1024;
        let transaction = TransactionRecord {
            transaction_type: "deposit".to_owned(),
            client: 1,
            tx: deposit_transaction_id,
            amount: Some("1".to_owned()),
        };

        assert_eq!(processor.process_transaction(transaction), Ok(()));
        let transaction = TransactionRecord {
            transaction_type: "dispute".to_owned(),
            client: 1,
            tx: deposit_transaction_id,
            amount: None,
        };

        assert_eq!(processor.process_transaction(transaction.clone()), Ok(()));
        assert!(processor.process_transaction(transaction).is_err());

        let transaction = TransactionRecord {
            transaction_type: "resolve".to_owned(),
            client: 1,
            tx: deposit_transaction_id,
            amount: None,
        };

        assert_eq!(processor.process_transaction(transaction.clone()), Ok(()));

        assert!(processor.process_transaction(transaction).is_err());

        for client in processor.clients.into_values() {
            assert_eq!(client.client_id(), ClientId(1));
            assert_eq!(client.total(), Amount::from_str("1.0".to_owned()).unwrap());
            assert_eq!(
                client.available(),
                Amount::from_str("1.0".to_owned()).unwrap()
            );
            assert_eq!(client.locked(), false);
            assert_eq!(client.held(), Amount::from_str("0.0".to_owned()).unwrap());
        }
    }

    // Test that withdraw after chargeback is not processed
    #[test]
    fn test_deposit_dispute_withdraw_chargeback_withdraw() {
        let mut processor = TransactionProcessor::<CACHE_SIZE_LIMIT, CACHE_SIZE_LINE>::new();
        let deposit_transaction_id = 8 * 1024;
        let transaction = TransactionRecord {
            transaction_type: "deposit".to_owned(),
            client: 1,
            tx: deposit_transaction_id,
            amount: Some("1".to_owned()),
        };

        assert_eq!(processor.process_transaction(transaction), Ok(()));
        let transaction = TransactionRecord {
            transaction_type: "dispute".to_owned(),
            client: 1,
            tx: deposit_transaction_id,
            amount: None,
        };
        assert_eq!(processor.process_transaction(transaction), Ok(()));

        let transaction = TransactionRecord {
            transaction_type: "withdrawal".to_owned(),
            client: 1,
            tx: deposit_transaction_id + 1,
            amount: Some("1".to_owned()),
        };
        assert!(processor.process_transaction(transaction).is_err());

        let transaction = TransactionRecord {
            transaction_type: "chargeback".to_owned(),
            client: 1,
            tx: deposit_transaction_id,
            amount: None,
        };

        assert_eq!(processor.process_transaction(transaction.clone()), Ok(()));

        assert!(processor.process_transaction(transaction).is_err());

        let transaction = TransactionRecord {
            transaction_type: "withdrawal".to_owned(),
            client: 1,
            tx: deposit_transaction_id + 1,
            amount: Some("1".to_owned()),
        };
        assert!(processor.process_transaction(transaction).is_err());

        for client in processor.clients.into_values() {
            assert_eq!(client.client_id(), ClientId(1));
            assert_eq!(client.total(), Amount::from_str("0.0".to_owned()).unwrap());
            assert_eq!(
                client.available(),
                Amount::from_str("0.0".to_owned()).unwrap()
            );
            assert_eq!(client.locked(), true);
            assert_eq!(client.held(), Amount::from_str("0.0".to_owned()).unwrap());
        }
    }
}
