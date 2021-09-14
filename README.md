## Reading the code source
- main.rs: - Reads the entries from stdin and initializes a TransactionProcessor and then it calls
  it for each TransactionRecord.
- processor.rs: It contains the main logic for the TransactionProcessor whose purpose is to interpret
  what transaction type we are processing and delegate the further processing of the transaction 
  to the Client objects referenced in the transaction.
- client.rs: It contains the main logic for processing a transaction for a given client, each client 
  keeps its own list of processed transactions and transaction which are disputed. 
- type_defs: It contains the definition of the types used to internally represent a ClientId, TransactionId, 
   Amount and Transaction. In order to avoid any mistake when dealing with these values decided to use specific
   domain types instead of using the backing types directly. This has the benefit that we use the compiler to validate
   that we are using the right type instead of accidentally passing the wrong parameters to function calls.
- transaction_cache: It contains the definition of a cache of transactions which could store the transaction either
  memory or to the disk in  case the threshold defined by CACHE_SIZE_LIMIT.

## Assumptions 
- Dispute transactions can reference only deposit transactions.
- After an account is locked no other transaction is processed.
- When transactions come with a transaction id that has been processed already we return an error and let 
  the main loop ignore the transactions.
- Dispute for a transaction already disputed returns error.
- Resolve and chargeback for a transaction not disputed returns error.
- Clients could have a negative balance accounts for the case when a deposit transaction is disputed after an 
  withdrawl has been processed.

## Testing file
  *tests/inputs/* -> Contains sample data files used for testing

  *tests/outputs/* -> Contains the expected output

  *transaction.rs* -> Contains the rust unittests used for validating the implementation.

## Design considerations:
**LargeDataSets**: For the situation where  we could not fit the whole dataset into the main memory the TransactionCache has been implemented in order to store part of the processed transactions to the disk and a load them back in memory in case the data is needed. The TransactionCache uses multiple HashMaps for all the transactions stored in memory and serialize those hashmaps using
serde in case the CACHE_SIZE_LIMIT is reached. In order to make sure we do not have to manipulate a single large data file into disk we use a CacheKey in order to split the list of transaction into multiple disjunct files(cache lines) and when we need to process a transaction that matches a given CacheKey we need to load only the data for that given CacheKey. The way the CacheKey 
is generated from a transaction id we aim to keep in the main memory only the list of most recent processed transaction(they highest known transaction ids) however that really depends on the pattern the transactions are generated.

However, even with the above considerations in mind the performance of the Cache would drasticaly depend on the real-usecase patterns, so further optimizing and fine tune of the CACHE_SIZE_LIMIT and CACHE_LINE_SIZE and the cache algorithm itself would be needed in order to get acceptable productions performances. The fine tunning would depend on multiple variables like resources available to the application, the access patterns and bandwidth and latency requirements.

**MultithreadEnvironment**: The current implementation is not multithread safe, luckly for us Rust would tell us that in case 
we want to move the modules into a multi-thread/async environment. However, extending the modules to also behave corretly in a concurent environment could be achieved with relative little effort, by using some locking primitives around each Client object.

**Decimal Precisions**: Opted to use rust-decimal in order to be able to frational digits with no round-off errors, the crate seems to 
actively maintained and it has many active downloads. However, in a production environment a thorough assement would have needed to 
be done in order to gain confidence in using it. 

## Things to improve
- Testing with more diverse data sets.
- Fine tune the TransactionCache and improve the caching algorithm.
