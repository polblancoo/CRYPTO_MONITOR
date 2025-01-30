use std::collections::HashSet;
use once_cell::sync::Lazy;

static TRADING_PAIRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut pairs = HashSet::new();
    pairs.insert("RUNEUSDT");
    pairs.insert("BTCUSDT");
    pairs.insert("ETHUSDT");
    pairs.insert("BNBUSDT");
    pairs.insert("ADAUSDT");
    pairs.insert("DOGEUSDT");
    pairs.insert("SOLUSDT");
    pairs.insert("DOTUSDT");
    pairs.insert("MATICUSDT");
    pairs.insert("AVAXUSDT");
    pairs
});

pub fn is_valid_pair(symbol: &str) -> bool {
    TRADING_PAIRS.contains(symbol)
}

pub fn get_similar_pairs(input: &str) -> Vec<&'static str> {
    TRADING_PAIRS
        .iter()
        .filter(|&&pair| pair.to_lowercase().contains(&input.to_lowercase()))
        .copied()
        .collect()
}

pub fn get_all_pairs() -> Vec<&'static str> {
    TRADING_PAIRS.iter().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pair() {
        assert!(is_valid_pair("RUNEUSDT"));
        assert!(!is_valid_pair("INVALIDPAIR"));
    }

    #[test]
    fn test_similar_pairs() {
        let similar = get_similar_pairs("RUN");
        assert!(similar.contains(&"RUNEUSDT"));
    }
}