use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct Coin {
    pub id: String,
    pub symbol: String,
    pub name: String,
}

pub async fn get_supported_coins() -> Result<Vec<Coin>, Box<dyn Error>> {
    let client = Client::new();
    let response = client
        .get("https://api.coingecko.com/api/v3/coins/list")
        .send()
        .await?
        .json::<Vec<Coin>>()
        .await?;
        
    Ok(response)
} 