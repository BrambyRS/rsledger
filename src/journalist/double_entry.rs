/*
TransactionAmount struct
*/
#[derive(Clone)]
pub struct TransactionAmount {
    amount: i32, // We save the amount as an integer (100x the amount in the journal) to avoid floating point issues.
    currency: String,
}

impl core::fmt::Display for TransactionAmount {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Format the amount as a decimal with 2 places by placing a decimal point before the last two digits.
        let amount_str = format!("{}.{:02}", self.amount / 100, self.amount.abs() % 100);
        write!(f, "{} {}", amount_str, self.currency)
    }
}

impl std::ops::Neg for TransactionAmount {
    type Output = Self;

    fn neg(self) -> Self::Output {
        TransactionAmount {
            amount: -self.amount,
            currency: self.currency,
        }
    }
}

impl PartialEq for TransactionAmount {
    fn eq(&self, other: &Self) -> bool {
        self.amount == other.amount && self.currency == other.currency
    }
}

impl TransactionAmount {
    pub fn from_str(amount_str: &str) -> Option<Self> {
        // Split the amount string into the numeric part and the currency part.
        let parts: Vec<&str> = amount_str.split_whitespace().collect();
        if parts.len() != 2 {
            return None;
        }

        let amount_part = parts[0];
        let currency_part = parts[1].to_string();

        // Remove the decimal point from the amount part and convert it to an integer.
        // If there is no decimal point, we can just parse it as an integer and multiply by 100.
        let amount_int: i32;
        if amount_part.contains('.') {
            let amount_int_str = amount_part.replace('.', "");
            amount_int = match amount_int_str.parse::<i32>() {
                Ok(val) => val,
                Err(_) => return None,
            };
        } else {
            let whole_part = match amount_part.parse::<i32>() {
                Ok(val) => val,
                Err(_) => return None,
            };
            amount_int = whole_part * 100; // Multiply by 100
        }

        Some(TransactionAmount {
            amount: amount_int,
            currency: currency_part,
        })
    }

    pub fn same_currency(&self, other: &Self) -> bool {
        self.currency == other.currency
    }

    pub fn same_amount(&self, other: &Self) -> bool {
        self.amount == other.amount
    }
}

/*
DoubleEntry struct
*/
pub struct DoubleEntry {
    date: String,
    description: String,
    account_1: String,
    amount_1: TransactionAmount,
    account_2: String,
    amount_2: TransactionAmount,
}

impl core::fmt::Display for DoubleEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{} {}\n\t{} {}\n\t{} {}\n\n", self.date, self.description, self.account_1, self.amount_1, self.account_2, self.amount_2)
    }
}

impl DoubleEntry {
    pub fn new (date: String, description: String, account_1: String, amount_1: TransactionAmount, account_2: String, amount_2: TransactionAmount) -> Self {
        DoubleEntry {
            date,
            description,
            account_1,
            amount_1,
            account_2,
            amount_2,
        }
    }
}