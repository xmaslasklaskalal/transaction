mod client;
mod processor;
mod transaction_cache;
mod type_defs;

use processor::{TransactionProcessor, CACHE_SIZE_LIMIT, CACHE_SIZE_LINE};
use std::env;
use type_defs::TransactionRecord;

use std::fs;

fn main() {
    let mut processor = TransactionProcessor::<CACHE_SIZE_LIMIT, CACHE_SIZE_LINE>::new();

    let args: Vec<String> = env::args().collect();
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .has_headers(false)
        .from_reader(fs::File::open(args[1].clone()).expect("Could not open input file"));

    for (index, result) in rdr.deserialize().enumerate() {
        match result {
            Ok(transaction_record) => {
                let copy: TransactionRecord = transaction_record;
                // Intentionally continue processing even in case of errors
                if let Err(err) = processor.process_transaction(copy.clone()) {
                    eprintln!("Ignoring error: {} for record: {:?}", err, copy);
                }
            }
            Err(err) => {
                // First entry might be the header, so it is expected that we might
                // not be able to convert it into a TransactionRecord.
                if index > 0 {
                    eprintln!("Ignoring error {}", err);
                }
            }
        }
    }

    processor
        .serialize()
        .expect("Could not serialize processor");
}
