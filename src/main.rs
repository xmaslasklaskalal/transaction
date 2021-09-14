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
        .from_reader(fs::File::open(args[1].clone()).expect("Could not open input file"));

    for result in rdr.deserialize() {
        match result {
            Ok(transaction_record) => {
                let copy: TransactionRecord = transaction_record;
                // Intentionally continue processing even in case of errors
                if let Err(err) = processor.process_transaction(copy.clone()) {
                    eprintln!("Ignoring error: {} for record: {:?}", err, copy);
                }
            }
            Err(err) => {
                eprintln!("Ignoring error {}", err);
            }
        }
    }

    processor
        .serialize()
        .expect("Could not serialize processor");
}
