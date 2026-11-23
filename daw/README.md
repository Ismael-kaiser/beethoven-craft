# Beethoven DAW

El estudio de música de Beethoven Craft: rack de instrumentos, piano roll, arreglador, mezclador, automatización, grabación y exportación (WAV, FLAC, MP3, OGG y MIDI). Carga plugins CLAP, VST3 y VST2. Funciona en Windows, macOS y Linux, en español o en inglés.

Normalmente se abre desde el menú de Beethoven Craft («Nueva canción»), que le pasa el idioma. Para compilarlo solo:

    cargo build --release -p beethoven_daw

El programa queda en `target/release/beethoven_daw`.

## Licencia

Software libre bajo la GNU General Public License, versión 3 o posterior (`LICENSE`), sin garantía. Los avisos de copyright están en el encabezado de cada archivo y en `THIRD-PARTY-NOTICES.txt`, junto con las licencias de las librerías de terceros.
