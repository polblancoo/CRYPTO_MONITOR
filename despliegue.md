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