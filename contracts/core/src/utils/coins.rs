use {
    cosmwasm_std::{Coin, OverflowError, Uint128},
    std::{collections::BTreeMap, fmt},
};

// denom => amount
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coins(BTreeMap<String, Uint128>);

// UNSAFE: because we don't check for duplicate denoms or zero amounts
// only use this for trusted coin vecs, such as MessageInfo::funds
impl From<Vec<Coin>> for Coins {
    fn from(coin_vec: Vec<Coin>) -> Self {
        Self(coin_vec
            .into_iter()
            .map(|coin| (coin.denom, coin.amount))
            .collect())
    }
}

// NOTE: the output vec is guaranteed to be ordered alphabetically ascendingly
// by the denoms
impl From<Coins> for Vec<Coin> {
    fn from(coins: Coins) -> Self {
        coins
            .0
            .into_iter()
            .map(|(denom, amount)| Coin {
                denom,
                amount,
            })
            .collect()
    }
}

impl fmt::Display for Coins {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "[]");
        }

        let s = self
            .0
            .iter()
            .map(|(denom, amount)| format!("{amount}{denom}"))
            .collect::<Vec<_>>()
            .join(",");
        write!(f, "{s}")
    }
}

impl Coins {
    pub fn empty() -> Self {
        Self(BTreeMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn add(&mut self, new_coin: Coin) -> Result<(), OverflowError> {
        let amount = self.0.entry(new_coin.denom).or_insert_with(Uint128::zero);
        *amount = amount.checked_add(new_coin.amount)?;
        Ok(())
    }
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use cosmwasm_std::coin;

    use super::*;

    #[test]
    fn adding() {
        let mut coins = Coins::empty();

        coins.add(coin(12345, "umars")).unwrap();
        coins.add(coin(23456, "uastro")).unwrap();
        coins.add(coin(34567, "uosmo")).unwrap();
        coins.add(coin(88888, "umars")).unwrap();

        let vec: Vec<Coin> = coins.into();

        assert_eq!(
            vec,
            vec![coin(23456, "uastro"), coin(12345 + 88888, "umars"), coin(34567, "uosmo")],
        );
    }

    #[test]
    fn comparing() {
        let coins1 = Coins::from(vec![
            coin(23456, "uastro"),
            coin(88888, "umars"),
            coin(34567, "uosmo"),
        ]);

        let mut coins2 = coins1.clone();
        assert_eq!(coins1, coins2);

        coins2.add(coin(1, "umars")).unwrap();
        assert_ne!(coins1, coins2);
    }
}
