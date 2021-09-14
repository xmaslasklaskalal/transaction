use serde::{Deserialize, Serialize};

use fmt::Display;
use rust_decimal::Decimal;
use std::fmt;
use std::ops::{AddAssign, SubAssign};
use std::str::FromStr;

/// Type to represent a client Id
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct ClientId(pub u16);

/// Type to represent a transaction Id
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct TransactionId(pub u32);

/// Decimal precision level
const PRECISION: u32 = 4;

/// Type to represent the amount held by a client account
#[derive(Copy, Debug, Clone, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount(Decimal);

impl Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

impl Amount {
    pub fn new() -> Self {
        Amount(Decimal::new(0, 4))
    }

    pub fn from_str(fixed_value: String) -> Result<Self, String> {
        let decimal = Decimal::from_str(&fixed_value).unwrap();
        if decimal.scale() > PRECISION {
            return Err("Invalid precision".to_owned());
        }

        Ok(Amount(decimal))
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

/// Type which holds a transaction information as read from the csv file.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionRecord {
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub client: u16,
    pub tx: u32,
    #[serde(default)]
    pub amount: Option<String>,
}

/// Type to represent a transaction.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Transaction {
    Deposit(ClientId, TransactionId, Amount),
    Withdrawal(ClientId, TransactionId, Amount),
    Dispute(ClientId, TransactionId),
    Resolve(ClientId, TransactionId),
    ChargeBack(ClientId, TransactionId),
    Unknown,
}

impl Transaction {
    pub fn from_record(record: TransactionRecord) -> Result<Self, String> {
        let transaction = match record.transaction_type.as_str() {
            "deposit" => Transaction::Deposit(
                ClientId(record.client),
                TransactionId(record.tx),
                Amount::from_str(record.amount.unwrap_or_else(|| "0.0".to_owned()))?,
            ),
            "withdrawal" => Transaction::Withdrawal(
                ClientId(record.client),
                TransactionId(record.tx),
                Amount::from_str(record.amount.unwrap_or_else(|| "0.0".to_owned()))?,
            ),
            "dispute" => Transaction::Dispute(ClientId(record.client), TransactionId(record.tx)),
            "resolve" => Transaction::Resolve(ClientId(record.client), TransactionId(record.tx)),
            "chargeback" => {
                Transaction::ChargeBack(ClientId(record.client), TransactionId(record.tx))
            }
            _ => Transaction::Unknown,
        };
        Ok(transaction)
    }
}
