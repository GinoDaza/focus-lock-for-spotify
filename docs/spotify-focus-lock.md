# Spotify Focus Lock

## Idea

Un modo de estudio para Spotify (cuenta gratis, escritorio) que deja **solo mis dos playlists de enfoque** y no se puede quitar hasta que pase el tiempo que yo elija.

## Problema

- La música divertida (phonk, etc.) me distrae cuando leo o programo.
- No quiero borrar Spotify ni usar otra cuenta.
- Tengo Premium en mi cuenta, pero no una segunda cuenta Premium solo para estudiar.
- No tengo Cold Turkey para apps, así que el candado tiene que sostenerse por sí solo.

## Qué hace

1. Elijo cuántas horas quiero el modo de enfoque.
2. Mientras dure, Spotify solo muestra:
   - Binaural Beats: Focus
   - Instrumental Studying (soft/no lyrics)
3. Sin buscador, sin biblioteca, sin inicio: no hay forma de llegar a otra música desde la interfaz.
4. No hay botón para desactivarlo hasta que termine el tiempo.
5. Al terminar, Spotify vuelve a la normalidad.

## Enfoque técnico (a grandes rasgos)

- **Spicetify**: herramienta open source que modifica el cliente de escritorio con CSS y JavaScript. Elegida porque vive dentro de Spotify y no deja un proceso aparte.
- Alternativa: script con la API oficial de Spotify (sirve porque tengo Premium), que pausa o redirige si suena algo fuera de las playlists. Descartada por ahora porque corre como proceso aparte.
- Una **extensión propia** que oculta la interfaz y guarda la hora de fin del candado.
- Un **script de instalación** para Windows.

## Pendiente

- Links de las dos playlists (Compartir → Copiar enlace) para sacar sus IDs.
- Instalar Spicetify en Windows.
- Decidir cómo se ve el selector de horas al activar el modo.
- Probar que sobreviva a las actualizaciones de Spotify.

## Cosas a tener en cuenta

- Modificar el cliente va contra los términos de Spotify, aunque es una práctica muy extendida.
- Es una herramienta de fricción pensada para mí, no un bloqueo a prueba de todo.
