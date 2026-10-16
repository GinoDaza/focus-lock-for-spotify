# Plan: Tunelock — candado de enfoque para Spotify de escritorio (Windows)

> **Estado (2026-10-16):** M0, M1 y M2 hechos; sigue M3. En la implementación hubo dos cambios respecto a este plan:
> - La persistencia es un archivo JSON en lugar de SQLite (ADR 0002).
> - La extensión no usa un overlay propio: aplica guardias de navegación y de reproducción más CSS estético (ADR 0003).
>
> El estado actual del proyecto está en `AGENTS.md`.

## Contexto

La música que distrae (phonk, etc.) rompe la concentración al estudiar o programar. Investigamos (nosotros y agy, ver `investigacion-agy.md`) y **no existe** una herramienta que reduzca la app de escritorio de Spotify a unas pocas playlists con un candado por tiempo estilo Cold Turkey. Los bloqueadores existentes bloquean apps o webs enteras; ninguno permite "deja la app, pero solo este contenido".

Objetivo: un proyecto **open source (GPL-3.0), gratis, llamado Tunelock**, con buena arquitectura y mantenimiento ligero. Alcance de v0.1: **Windows + app de escritorio de Spotify**. Navegador y reproductor web quedan fuera (decisión del usuario).

## Decisiones de stack (lo estándar para cada pieza)

| Pieza | Tecnología | Por qué |
|---|---|---|
| Núcleo + servicio | **Rust**, crate `windows-service`, `tokio` | Binario nativo pequeño, sin runtime. `windows-service` es el crate estándar para servicios Win32. |
| IPC servicio ↔ UI | Named pipe (`interprocess`) con ACL + protocolo JSON tipado (`serde`) | Es el mecanismo estándar en Windows entre un servicio y una app de usuario. La ACL controla quién puede enviar comandos. |
| Persistencia | SQLite (`rusqlite`, migraciones) en `%ProgramData%\Tunelock`, con ACL solo para SYSTEM/Admin | Bloqueos, presets, horarios e historial. Escritura atómica. |
| UI de escritorio | **Tauri v2** + React + TypeScript + Vite, con icono en la bandeja | Estándar actual para apps de escritorio con Rust. Reutiliza los tipos del protocolo. |
| Extensión de Spotify | TypeScript empaquetado con **esbuild** en un solo `.js` de Spicetify | Las extensiones de Spicetify son un único archivo JS (doc oficial). |
| Instalador | Bundler NSIS de Tauri (`perMachine`) + `installer_hooks` (.nsh) | Los hooks de NSIS instalan y desinstalan el servicio. |
| CI/CD | GitHub Actions `windows-latest`. Firma con SignPath Foundation (gratis para OSS, requiere una release publicada y build automatizado) | Estándar para OSS en Windows. |

## Arquitectura

```
tunelock/
├─ Cargo.toml                 (workspace)
├─ crates/
│  ├─ core/       dominio puro: Lock, Preset, Schedule, Allowlist, máquina de estados, reloj abstracto
│  ├─ protocol/   tipos de mensajes IPC (serde), versionados
│  ├─ service/    binario `tunelock-service`: servicio Windows, IPC, persistencia, enforcers
│  └─ cli/        `tunelock` CLI de desarrollo/diagnóstico (status, start lock, presets)
├─ apps/desktop/  Tauri v2 (src-tauri/ + src/ React)
├─ extension/     Spicetify: src/*.ts → dist/tunelock.js (esbuild)
├─ installer/     hooks .nsh
├─ docs/          idea original, investigación, ADRs (decisiones de arquitectura)
└─ .github/workflows/
```

### Núcleo (`crates/core`)
- **Máquina de estados del bloqueo:** `Idle → Active{Flexible|Locked} → Expired`. Un bloqueo en modo `Locked` rechaza cualquier comando de cancelación hasta su fin.
- **Tiempo resistente a cambios del reloj:** el servicio cuenta el tiempo restante con un reloj monotónico y lo persiste periódicamente. Cambiar la hora del sistema no adelanta el fin. Se reanuda correctamente tras reinicio o suspensión.
- **Presets** ("Estudio 10 h", "Semana de exámenes 5 días") y **horarios** (L–V 9:00–18:00), modelados ya en v0.1 aunque la UI de horarios llegue en v0.2.
- **Allowlist** por perfil: lista de URIs `spotify:playlist:<id>`.
- Pura y sin I/O: se prueba al 100 % con un reloj falso.

### Servicio (`crates/service`)
- Corre como **LocalSystem** y arranca automáticamente con recuperación configurada (reinicio si se cae).
- **Servidor IPC** (named pipe): la UI y la CLI envían comandos (`StartLock`, `GetStatus`, `ListPresets`…).
- **Endpoint local de solo lectura** (WebSocket/HTTP en `127.0.0.1`): la extensión consulta el estado del bloqueo y la allowlist, y envía un heartbeat. Como es de solo lectura, exponerlo no permite desbloquear nada.
- **Enforcers** como trait `Enforcer { on_lock_start, on_tick, on_lock_end }`. Generalizar a otras apps más adelante consiste en agregar un enforcer.
  - `SpotifyEnforcer`:
    1. Al iniciar el bloqueo, verifica que Spicetify esté aplicado con la extensión de Tunelock. Si no lo está, la reaplica en el contexto del usuario.
    2. **Fail-closed:** si `Spotify.exe` corre durante el bloqueo y la extensión no envía heartbeat en N segundos, cierra Spotify. Encaja con el criterio del usuario: "prefiero no escuchar nada".
    3. Protege con ACL los archivos de la extensión mientras dure el bloqueo y restaura los permisos al terminar.
  - Detecta las actualizaciones de Spotify (que borran Spicetify) y vuelve a aplicarlo.
- La desinstalación y la detención del servicio se rechazan mientras haya un bloqueo `Locked` activo (el hook de NSIS consulta al servicio).

### Extensión (`extension/`)
- **Vista de enfoque como overlay propio** a pantalla completa, con las playlists permitidas y los controles del reproductor. Se prefiere a ocultar decenas de selectores con CSS porque se rompe mucho menos cuando Spotify cambia su interfaz.
- **Guardia de navegación** (`Spicetify.Platform.History.listen`): cualquier ruta fuera de la allowlist redirige a la vista de enfoque.
- **Guardia de reproducción** (`Spicetify.Player` evento `songchange`): si el contexto que suena no está en la allowlist, reproduce una playlist permitida o pausa.
- Los selectores y rutas internas de Spotify van en un solo archivo `spotify-compat.ts`, para que adaptarse a una actualización toque un solo lugar.
- Sin bloqueo activo, la extensión no hace nada y Spotify funciona normal.

### UI (`apps/desktop`)
- Icono en la bandeja con estado y cuenta regresiva.
- Pantallas: Inicio (iniciar bloqueo, flexible o candado, con duración en horas o días), Presets, Playlists permitidas (pegar el link "Copiar enlace" y extraer el ID), Ajustes.
- Confirmación fuerte antes de un bloqueo `Locked` ("no podrás cancelarlo hasta…").
- Si cierras la UI, el bloqueo sigue (vive en el servicio).

## Riesgos conocidos (para resolver con spikes en la fase 0)
1. **¿La extensión puede conectarse a `127.0.0.1` desde el cliente de Spotify?** No lo confirmé; la doc no lo aclara. Alternativa si no puede: el servicio genera la config de la extensión con el estado embebido, vuelve a aplicarla y reinicia Spotify al iniciar o terminar un bloqueo. El heartbeat se reemplaza por una verificación de los archivos aplicados.
2. **Spotify se instala por usuario** (`%APPDATA%`), pero el servicio corre como SYSTEM. Para aplicar Spicetify hay que lanzar procesos en la sesión del usuario activo (`WTSQueryUserToken` + `CreateProcessAsUser`).
3. **La versión de Spotify de Microsoft Store no es compatible con Spicetify.** El instalador debe detectarla y pedir la versión de spotify.com.
4. **Ningún candado es absoluto para un administrador local.** El objetivo es la fricción alta (como Cold Turkey), no la invulnerabilidad. Se documenta así en el README, sin detallar cómo evadirlo.
5. **Términos de servicio de Spotify:** modificar el cliente va contra ellos. Se avisa en el README.
6. **Entorno:** se desarrolla en WSL, pero el servicio y Tauri son de Windows. Se compila con `cargo-xwin` o del lado Windows, y la CI usa `windows-latest`. Las pruebas reales se hacen en Windows (idealmente en una VM o con una cuenta de prueba).

## Hitos

- **M0 — Repo y spikes**
  - `git init`, `.gitignore` (Rust, Node, Tauri), `LICENSE` GPL-3.0, `README.md`, `.editorconfig`.
  - Mover `spotify-focus-lock.md` e `investigacion-agy.md` a `docs/`.
  - Primer ADR: decisión del stack.
  - Spikes de los riesgos 1 y 2.
- **M1 — Núcleo + servicio + CLI:** máquina de estados, persistencia, IPC, servicio instalable, y `tunelock lock start --hours 2 --locked` funcionando de punta a punta.
- **M2 — Extensión:** overlay, guardias, conexión con el servicio.
- **M3 — SpotifyEnforcer:** aplicar y reaplicar Spicetify, watchdog fail-closed, ACL, detección de actualizaciones.
- **M4 — UI Tauri:** bandeja, iniciar bloqueo, presets, allowlist.
- **M5 — Release v0.1:** instalador NSIS con hooks del servicio, CI (fmt, clippy, tests, build), release en GitHub. Después, solicitud a SignPath.
- **v0.2:** UI de horarios programados, historial de sesiones, varios perfiles.

## Convenciones del repo
- Conventional Commits; rama `main` protegida más adelante.
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `pnpm lint`, `pnpm test`, y build de Tauri y de la extensión.
- README: qué es, capturas, instalación, aviso de los términos de Spotify, "proyecto personal sin garantías de soporte" y política de firma de código (requisito de SignPath).

## Verificación
- **Unitarias (core):** transiciones de estado, rechazo de cancelación en modo `Locked`, reanudación tras reinicio, cambio del reloj del sistema sin efecto, cálculo de horarios. Todo con reloj falso.
- **Integración (service):** cliente IPC de prueba contra el servicio en modo consola (sin instalarlo como servicio).
- **Extensión:** pruebas de la lógica de allowlist y guardias con Vitest (con `Spicetify` simulado).
- **End-to-end manual en Windows:** instalar → configurar 2 playlists → bloqueo `Locked` de 5 min → comprobar que solo se ven y suenan esas playlists, que cerrar la UI no lo quita y que reiniciar el PC lo mantiene → al expirar, Spotify vuelve a la normalidad.
