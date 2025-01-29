# Crypto Monitor Bot 🤖💰

Un bot de Telegram para monitorear precios de criptomonedas y recibir alertas cuando alcancen ciertos valores.

## Características ✨

- 📈 Monitoreo en tiempo real de precios de criptomonedas usando CoinGecko API
- 🔔 Alertas personalizadas por precio (arriba/abajo)
- 🔐 Sistema de autenticación seguro con Argon2
- 🔑 API keys para integración con otros servicios
- 📱 Notificaciones instantáneas vía Telegram
- 💾 Base de datos SQLite para persistencia

## Instalación y Configuración 🚀

### Prerrequisitos

- Rust 1.70+
- SQLite 3
- Token de Bot de Telegram
- API Key de CoinGecko

### Configuración del Entorno (.env)

Crea un archivo `.env` en la raíz del proyecto:

```env
# Base de Datos
DATABASE_URL=sqlite:./data/crypto_monitor.db

# API Keys
TELEGRAM_BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrsTUVwxyz  # De @BotFather
COINGECKO_API_KEY=CG-XXXXXXXXXXXXXXXXXXXXXXX              # De CoinGecko

# Configuración
CHECK_INTERVAL=60        # Intervalo en segundos
LOG_LEVEL=info          # debug, info, warn, error
```

### Instalación Local

```bash
# Clonar repositorio
git clone https://github.com/tu-usuario/crypto-monitor
cd crypto-monitor

# Compilar
cargo build --release

# Ejecutar
./target/release/crypto-monitor
```

## Uso del Bot 📱

### Comandos Disponibles

- `/start` - Inicia el bot
- `/help` - Muestra ayuda
- `/register <username> <password>` - Registra usuario
- `/alert <symbol> <price> <above|below>` - Crea alerta
- `/alerts` - Lista alertas activas
- `/delete <id>` - Elimina una alerta
- `/symbols` - Muestra criptomonedas soportadas

### Ejemplos

```bash
# Registro
/register cryptouser secretpass

# Crear alerta
/alert BTC 45000 above

# Listar alertas
/alerts
```

## Despliegue 🚀

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /usr/src/app/target/release/crypto-monitor /usr/local/bin/
COPY .env .
CMD ["crypto-monitor"]
```

```bash
docker build -t crypto-monitor .
docker run -d --name crypto-bot crypto-monitor
```

### VPS/Servidor

1. Preparar servidor:
```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential pkg-config libssl-dev sqlite3 libsqlite3-dev
```

2. Instalar Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

3. Configurar servicio:
```ini
# /etc/systemd/system/crypto-monitor.service
[Unit]
Description=Crypto Monitor Bot
After=network.target

[Service]
Type=simple
User=crypto-bot
WorkingDirectory=/home/crypto-bot/crypto-monitor
Environment=RUST_LOG=info
ExecStart=/home/crypto-bot/crypto-monitor/target/release/crypto-monitor
Restart=always

[Install]
WantedBy=multi-user.target
```

## Mantenimiento 🔧

### Logs
```bash
# Ver logs
sudo journalctl -u crypto-monitor -f
```

### Backup
```bash
# DB backup
sqlite3 data/crypto_monitor.db ".backup 'backup.db'"
```

### Actualización
```bash
sudo systemctl stop crypto-monitor
git pull
cargo build --release
sudo systemctl start crypto-monitor
```

## Contribuir 🤝

1. Fork el proyecto
2. Crea tu rama (`git checkout -b feature/AmazingFeature`)
3. Commit tus cambios (`git commit -m 'Add AmazingFeature'`)
4. Push a la rama (`git push origin feature/AmazingFeature`)
5. Abre un Pull Request

## Licencia 📝

Este proyecto está bajo la Licencia MIT - ver el archivo [LICENSE](LICENSE) para detalles.

## Contacto 📧

Tu Nombre - [@tu_twitter](https://twitter.com/tu_twitter)
GitHub: [tu-usuario](https://github.com/tu-usuario)

Proyecto: [https://github.com/tu-usuario/crypto-monitor](https://github.com/tu-usuario/crypto-monitor)

## FAQ ❓

### ¿Cómo obtengo un token de Telegram?
1. Habla con [@BotFather](https://t.me/botfather) en Telegram
2. Usa el comando `/newbot`
3. Sigue las instrucciones y guarda el token

### ¿Cómo obtengo una API key de CoinGecko?
1. Regístrate en [CoinGecko](https://www.coingecko.com/en/api)
2. Ve a tu panel de control
3. Genera una nueva API key

### ¿Por qué SQLite?
SQLite es perfecto para esta aplicación porque:
- No requiere servidor
- Fácil backup
- Excelente rendimiento para cargas pequeñas/medianas
- Zero-config

## Arquitectura 🏗️

### Componentes Principales

```plaintext
+-------------+     +-----------+     +-------------+
|  Telegram   | --> |   Bot     | --> |  Monitor    |
|   API       |     | Handler   |     |  Service    |
+-------------+     +-----------+     +-------------+
                         |                  |
                         v                  v
                    +-----------+     +-------------+
                    | Database  | <-- | CoinGecko   |
                    |  Layer    |     |    API      |
                    +-----------+     +-------------+
```

### Flujo de Datos
1. Usuario envía comando al bot
2. Bot Handler procesa el comando
3. Se consulta/actualiza la base de datos
4. Monitor Service verifica precios
5. Se envían notificaciones si necesario

## Ejemplos Detallados 📝

### Registro de Usuario
```bash
/register cryptouser secretpass
```
Respuesta:
```
✅ Registro exitoso!
Tu API key es: abc123def456...
Guárdala en un lugar seguro.
```

### Crear Alerta
```bash
/alert BTC 45000 above
```
Respuesta:
```
✅ Alerta creada exitosamente!
Símbolo: BTC
Precio objetivo: $45000.00
Condición: Above
```

### Listar Alertas
```bash
/alerts
```
Respuesta: 
```

## Opciones de Despliegue Gratuito 🆓

### 1. Oracle Cloud Free Tier
- **Características**:
  - 2 VMs AMD con 1 GB RAM
  - 4 vCPUs en total
  - 200 GB almacenamiento
  - IP pública gratuita
  - Sin límite de tiempo
- **Pasos**:
  1. Regístrate en [Oracle Cloud](https://www.oracle.com/cloud/free/)
  2. Crea una VM con Ubuntu
  3. Sigue las instrucciones de instalación VPS del README
  4. Perfecto para este bot por los recursos y estabilidad

### 2. Google Cloud Platform (Free Tier)
- **Características**:
  - e2-micro (2 vCPUs compartidas, 1 GB RAM)
  - 30 GB almacenamiento
  - Válido por 90 días + algunos servicios siempre gratuitos
- **Pasos**:
  1. Regístrate en [Google Cloud](https://cloud.google.com/free)
  2. Crea una VM e2-micro con Ubuntu
  3. Usa la IP externa fija (gratuita)

### 3. Railway.app
- **Características**:
  - 500 horas gratis/mes
  - 512 MB RAM
  - Despliegue automático desde GitHub
  - Soporte para Rust
- **Pasos**:
  1. Conecta tu repo de GitHub
  2. Configura las variables de entorno
  3. Railway detectará automáticamente el proyecto Rust

### 4. Fly.io
- **Características**:
  - 3 VMs compartidas (256 MB RAM)
  - 3 GB almacenamiento
  - IPv6 gratuita
- **Pasos**:
  ```bash
  # Instalar flyctl
  curl -L https://fly.io/install.sh | sh
  
  # Login y despliegue
  fly auth login
  fly launch
  fly secrets set TELEGRAM_BOT_TOKEN=xxx
  fly secrets set COINGECKO_API_KEY=xxx
  fly deploy
  ```

### Recomendación 👍
Para este bot específicamente, recomiendo **Oracle Cloud Free Tier** porque:
- Recursos suficientes (1 GB RAM)
- IP pública estable
- Sin límite de tiempo
- Almacenamiento para la base SQLite
- Permite ejecutar en background
- No requiere modificaciones al código

### Consideraciones de Despliegue 🤔
1. **Base de Datos**: 
   - SQLite funciona bien en todos estos servicios
   - Configura backups periódicos

2. **Variables de Entorno**:
   - Configúralas en el panel de control del servicio
   - No las incluyas en el código

3. **Logs**:
   - Usa `RUST_LOG=info` para debugging
   - Considera un servicio de logs externo

4. **Monitoreo**:
   - Configura health checks
   - Usa el sistema de alertas del proveedor
