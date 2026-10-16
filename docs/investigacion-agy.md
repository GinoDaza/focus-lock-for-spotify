# Investigación: Modo Enfoque Restringido para Spotify (Estilo Cold Turkey)

## 1. Veredicto

**NO EXISTE** (como solución integrada/llave en mano para el cliente de escritorio de Spotify; **EXISTE PARCIALMENTE** si se consideran combinaciones fragmentadas entre bloqueadores de URL web y el reproductor de navegador, pero carecen de la experiencia en la aplicación nativa de escritorio y presentan fugas de navegación).

> **Justificación en una línea:** No existe actualmente ningún software ni mod que aplique una lista blanca estricta de reproducción (solo 2 playlists) dentro del cliente oficial de Spotify de escritorio y que cuente con un sistema de candado temporal anti-manipulación.

---

## 2. Tabla Comparativa de Soluciones y Herramientas Evaluadas

| Nombre | Enlace | Qué hace | Qué le falta respecto a tu lista | Open Source | Última actualización | Windows | Dificultad para deshacer |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Spicetify CLI + Extensiones (`focusMode`, `spotify-control-toolkit`)** | [spicetify/cli](https://github.com/spicetify/cli)<br>[focus-mode-spicetify](https://github.com/fatihkaan22/focus-mode-spicetify)<br>[spotify-control-toolkit](https://github.com/SDavid33/spotify-control-toolkit) | Inyecta JS/CSS en el cliente de escritorio oficial en Windows. Puede ocultar barras de navegación y saltar canciones/artistas vetados. | No implementa lista blanca exclusiva para 2 playlists; no tiene temporizadores, programación ni modo "candado"; no resiste manipulación (corre en espacio de usuario). | Sí (LGPL-2.1 / MIT) | Activo (2025/2026) | Sí | **Bajo** (se puede desactivar fácilmente desde la consola o reinstalando el cliente). |
| **Cold Turkey Blocker** | [getcoldturkey.com](https://getcoldturkey.com/) | Bloqueador con temporizador, horarios y modo candado extremo (anti-manipulación). Bloquea ejecutables completos y URLs en navegadores. | No puede filtrar ni aplicar listas blancas dentro del cliente de escritorio (`Spotify.exe`, al ser una app Chromium/CEF). No modifica la interfaz de Spotify. Código cerrado. | No (Propietario / Comercial) | Activo (2025/2026) | Sí | **Alto** (resiste cierre de procesos, desinstalación y alteraciones del sistema). |
| **Axorith** | [axorithlabs/axorith](https://github.com/axorithlabs/axorith) | Alternativa moderna FOSS a Cold Turkey en .NET 10 / C# con arquitectura cliente-servidor (servicio de fondo `Axorith.Host`), presets y bloqueos. | Bloquea ejecutables enteros o dominios; no puede inspeccionar o limitar pantallas dentro de la app de escritorio de Spotify; no tiene modding de interfaz. | Sí / Source-Available (BSL 1.1) | Activo (2025/2026) | Sí | **Medio** (su servicio desacoplado sobrevive al cierre de la UI, pero carece de ganchos de protección a nivel de sistema). |
| **FocusMe** | [focusme.com](https://focusme.com/) | Bloqueador de distracciones con "Forced Mode" (candado), horarios programados y control de apps y sitios web. | Código cerrado y de pago. Solo bloquea el proceso `Spotify.exe` en su totalidad o URLs en navegadores web soportados; no puede limitar funciones internas de Spotify. | No (Propietario) | Activo (2025/2026) | Sí | **Alto** (mecanismo anti-tamper robusto mediante servicios del sistema). |
| **LeechBlock NG** | [proginosko/LeechBlockNG](https://github.com/proginosko/LeechBlockNG) | Extensión de navegador para listas blancas/negras de URLs, horarios y modo "Lockdown" (candado temporal estricto). | Funciona únicamente dentro del navegador web; no tiene ningún control sobre el cliente de escritorio de Spotify; no oculta recomendaciones internas del reproductor web. | Sí (MPL-2.0) | Activo (2025/2026) | Sí (vía navegador) | **Medio** (se puede evadir utilizando otro navegador o perfiles sin bloqueo). |
| **Cold Turkey Micromanager** | [getcoldturkey.com/micromanager](https://getcoldturkey.com/micromanager/) | Restringe el uso de Windows exclusivamente a una lista blanca de aplicaciones abiertas. | Actúa a nivel de ventana/ejecutable. Dentro de Spotify permite acceso total (buscador, biblioteca, canciones no deseadas). No es open source. | No (Propietario) | Activo (2024/2025) | Sí | **Alto** (usa la misma base anti-manipulación que Cold Turkey Blocker). |
| **Freedom** | [freedom.to](https://freedom.to/) | Bloqueador comercial multiplataforma con "Locked Mode" y sesiones programadas. | Bloquea procesos completos o tráfico web a nivel general. No puede aplicar una lista blanca interna dentro del cliente de escritorio de Spotify. | No (Propietario) | Activo (2025/2026) | Sí | **Medio / Alto** (depende del nivel de permisos otorgados en Windows). |
| **Spotube** | [KRTirtho/spotube](https://github.com/KRTirtho/spotube) | Cliente de escritorio open source y ligero para Spotify (Flutter). | No posee modo quiosco, listas blancas, temporizadores ni candados. Cuenta con buscador y catálogo completo. | Sí (GPL-3.0 / BSD-4-Clause) | Activo (2025/2026) | Sí | **Bajo** (se puede cerrar o cambiar de app sin fricción). |
| **Spotify Official (Idea Submissions)** | [Spotify Community Ideas](https://community.spotify.com/t5/Idea-Submissions/) | Hilos en foros oficiales solicitando "Kiosk Mode", "Whitelist de playlists" y "Study Mode". | Spotify no lo ha implementado. La compañía cerró las propuestas indicando que no está en sus planes inmediatos o derivó a filtros estándar de contenido explícito. | No (Código cerrado) | Descartado por Spotify | N/A | N/A |

---

## 3. Los 3 Mejores Candidatos para Adaptar o Tomar de Base

### 1. Spicetify CLI ([spicetify/cli](https://github.com/spicetify/cli))
* **Por qué es candidato clave:** Es la **única** herramienta open source madura capaz de alterar el cliente oficial de escritorio de Spotify en Windows sin requerir la descompilación o parcheo manual de binarios ejecutables. 
* **Qué aporta:** Permite inyectar scripts en JavaScript y estilos en CSS directamente en el entorno interno de Spotify (Chromium Embedded Framework). Mediante su API interna (`Spicetify.Platform.History` y `Spicetify.Player`), se puede suprimir toda la navegación visual (ocultar barra de búsqueda, inicio, biblioteca y paneles laterales) y forzar la permanencia estricta en los identificadores URI de las 2 playlists deseadas.
* **Limitación a resolver:** No cuenta con sistema de candado ni resistencia a manipulaciones; requiere ser complementado con un guardián externo a nivel de sistema operativo.

### 2. Axorith ([axorithlabs/axorith](https://github.com/axorithlabs/axorith))
* **Por qué es candidato clave:** Provee el diseño de arquitectura open source (.NET / C# para Windows) más cercano a lo que se necesita para el "candado": un servicio en segundo plano desacoplado de la interfaz (`Axorith.Host`).
* **Qué aporta:** Su estructura permite gestionar temporizadores de sesión, horarios recurrentes (ej. Lun-Vie 9:00 a 18:00) y ejecución headless independiente de la ventana de usuario. Puede reutilizarse o tomarse de referencia para programar el daemon guardián que mantenga activas las restricciones temporales.
* **Limitación a resolver:** Actualmente está concebido para bloquear ejecutables completos (`.exe`), por lo que su lógica debe redirigirse para controlar el estado de la extensión de Spicetify y los permisos de archivo en Windows.

### 3. LeechBlock NG ([proginosko/LeechBlockNG](https://github.com/proginosko/LeechBlockNG))
* **Por qué es candidato clave:** Aunque es una extensión de navegador (MPL-2.0), su código contiene la lógica más refinada y probada de temporizadores, programación de días/horas y el algoritmo del modo **Lockdown** (bloqueo irrevocable que impide cancelar la sesión activa antes de que expire el tiempo).
* **Qué aporta:** Su modelo de datos en JavaScript (cálculo de ventanas de tiempo, control de cuenta atrás, estados de excepción y activación con un clic) es 100% reutilizable para el módulo de control temporal que se integre en la extensión de Spotify.

---

## 4. Arquitectura Propuesta: Cómo Construirlo Combinando Piezas Open Source

Dado que ninguna herramienta cubre simultáneamente la restricción de interfaz en Spotify Desktop y el candado temporal anti-manipulación, la solución óptima pasa por combinar **dos componentes complementarios**:

```
 ┌─────────────────────────────────────────────────────────────┐
 │                    SISTEMA OPERATIVO WINDOWS                │
 │                                                             │
 │  ┌───────────────────────────────────────────────────────┐  │
 │  │        SERVICIO GUARDIÁN LOCAL (Background Service)   │  │
 │  │        - Basado en arquitectura Axorith (.NET / Rust) │  │
 │  │        - Gestiona el tiempo y el "Modo Candado"       │  │
 │  │        - Bloquea permisos de edición (ACLs)           │  │
 │  │        - Bloquea reproductor web (hosts: open.spotify)│  │
 │  └──────────────────────────┬────────────────────────────┘  │
 │                             │ Comunicación (Lock State)     │
 │                             ▼                               │
 │  ┌───────────────────────────────────────────────────────┐  │
 │  │             SPOTIFY DESKTOP (CLIENTE OFICIAL)         │  │
 │  │  ┌─────────────────────────────────────────────────┐  │  │
 │  │  │      EXTENSIÓN SPICETIFY (Lógica de Interfaz)   │  │  │
 │  │  │  - CSS: Oculta Search, Home, Library, Feed      │  │  │
 │  │  │  - JS: Intercepta rutas (solo 2 Playlist URIs)  │  │  │
 │  │  │  - JS: Si suena canción fuera de lista -> pausa │  │  │
 │  │  │  - Algoritmo de temporizador (estilo LeechBlock)│  │  │
 │  │  └─────────────────────────────────────────────────┘  │  │
 │  └───────────────────────────────────────────────────────┘  │
 └─────────────────────────────────────────────────────────────┘
```

### Componente A: La "Cárcel" de Interfaz (Extensión Spicetify)
1. **Reglas de Estilo CSS Inyectadas:**
   * Ocultan mediante `display: none !important` los selectores del DOM correspondientes a:
     * Barra de búsqueda superior (`header`, `input[data-testid="search-input"]`).
     * Menú de inicio y exploración (`ul[data-testid="side-nav-bar"]`).
     * Biblioteca completa y recomendaciones del feed principal.
     * Enlaces a artistas o álbumes dentro de las filas de canciones para evitar desvíos.
   * Deja visibles únicamente: la carátula, los controles de reproducción y la vista principal fijada en las dos listas de reproducción.
2. **Controlador de Rutas y Reproducción en JavaScript:**
   * **Control de Navegación:** Se conecta a `Spicetify.Platform.History.listen()`. Si se detecta un intento de navegación a una URL distinta a `spotify:playlist:ID_BINAURAL` o `spotify:playlist:ID_INSTRUMENTAL`, se cancela la acción y se redirige a la lista activa.
   * **Control de Audio (Watchdog de Canciones):** Se conecta a `Spicetify.Player.addEventListener("songchange", callback)`. Si una canción iniciada no forma parte del contexto de las dos playlists aprobadas, se invoca inmediatamente `Spicetify.Player.pause()` o se reanuda la playlist autorizada.

### Componente B: El "Candado" Anti-Manipulación (Servicio Local en Windows)
Para evitar que el bloqueo sea trivialmente cancelado durante un momento de debilidad:
1. **Servicio en segundo plano (Background Worker):**
   * Corre con privilegios de sistema (`LocalSystem` o servicio estándar de Windows) basado en la arquitectura de **Axorith**.
   * Registra la marca de tiempo de finalización del bloqueo en un almacén local.
   * Ofrece dos modos:
     * **Modo Flexible:** Permite pausar o cancelar el bloqueo en cualquier momento con un clic.
     * **Modo Candado:** Ignora cualquier petición de desbloqueo hasta que el reloj del sistema alcance la hora programada.
2. **Protección contra evasiones comunes:**
   * **Bloqueo del reproductor web:** Durante el candado, el servicio añade una regla temporal en el archivo `hosts` de Windows (`C:\Windows\System32\drivers\etc\hosts`) que apunta `open.spotify.com` a `127.0.0.1`. Esto impide eludir el límite acudiendo a Chrome o Edge.
   * **Protección de la extensión:** El servicio aplica restricciones de permisos en el sistema de archivos (ACLs de Windows mediante `icacls`) sobre la carpeta de configuración de Spicetify, denegando temporalmente la modificación o borrado de la extensión al usuario activo.
   * **Persistencia:** Al ejecutarse Spotify, la extensión lee el estado del servicio local. Si el candado está vigente, la interfaz restringida se activa automáticamente sin posibilidad de interacción manual. Al vencer el tiempo, el servicio retira las restricciones y Spotify regresa a su modo normal sin reiniciar la aplicación.
