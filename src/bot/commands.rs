use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "Estos son los comandos disponibles:"
)]
pub enum Command {
    #[command(description = "muestra este mensaje")]
    Help,
    #[command(description = "inicia el bot")]
    Start,
    #[command(description = "registra tu usuario - /register <username> <password>")]
    Register { text: String },
    #[command(description = "crea una alerta de precio")]
    Alert,
    #[command(description = "crea alerta de depeg")]
    Depeg,
    #[command(description = "crea alerta de par")]
    PairDepeg,
    #[command(description = "lista tus alertas activas")]
    Alerts,
    #[command(description = "elimina una alerta")]
    Delete,
    #[command(description = "muestra los símbolos soportados")]
    Symbols,
    #[command(description = "Ver balance")]
    Balance { text: String },
    #[command(description = "Conectar exchange")]
    Connect { text: String },
    #[command(description = "Comprar")]
    Buy { text: String },
    #[command(description = "Vender")]
    Sell { text: String },
    #[command(description = "Ver órdenes")]
    Orders { text: String },
    #[command(description = "Crear orden")]
    Order(String),
    #[command(description = "Cancelar orden - /cancel <order_id>")]
    Cancel { text: String },
} 