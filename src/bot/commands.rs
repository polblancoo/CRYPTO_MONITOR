#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "Comandos disponibles:"
)]
pub enum Command {
    // ... comandos existentes ...
    
    #[command(description = "conectar exchange - /connect <exchange> <api_key> <api_secret> [passphrase]")]
    Connect { text: String },
    
    #[command(description = "comprar - /buy <exchange> <symbol> <quantity> [price]")]
    Buy { text: String },
    
    #[command(description = "vender - /sell <exchange> <symbol> <quantity> [price]")]
    Sell { text: String },
    
    #[command(description = "ver balance - /balance [exchange] [symbol]")]
    Balance { text: String },
    
    #[command(description = "ver órdenes abiertas - /orders [exchange] [symbol]")]
    Orders { text: String },
    
    #[command(description = "cancelar orden - /cancel <exchange> <order_id>")]
    Cancel { text: String },
    
    #[command(description = "mostrar este mensaje")]
    Help,
    
    #[command(description = "crear orden - /order <symbol> <side> <type> <quantity> [price]")]
    Order(String),
} 