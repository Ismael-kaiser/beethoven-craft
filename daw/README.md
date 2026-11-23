# Beethoven DAW

El estudio de música de Beethoven Craft. Funciona en Windows, macOS y Linux, en español o en inglés.

Todo está en pestañas que se abren, se cierran y se cambian como en un navegador:

- **Arreglo**: las partes de la canción en el tiempo, con clips, audio y automatización.
- **Ritmo**: los instrumentos y sus pasos, con 16 ritmos listos (rock, cumbia, reggaeton, banda, corrido tumbado, trap…) y un repartidor de golpes.
- **Notas**: el piano roll del instrumento elegido.
- **Mezcla**: niveles, efectos y envíos.
- **Partitura**: se compone escribiendo en el pentagrama; es la misma música que el piano roll.
- **Ideas**: acordes de la tonalidad, qué acorde suele seguir, progresiones por ánimo y generadores de bajo, arpegio, melodía y segunda voz.
- **Exportar**: WAV, FLAC, MP3, OGG, MIDI y MusicXML (MuseScore, Sibelius, Finale, Dorico).

**Llevar un ritmo al arreglo**: abajo de Ritmo, arrastra el patrón hasta la pestaña Arreglo y suéltalo en una pista, o usa «Poner en el arreglo».

**Escribir en la partitura**: con «Instrumento» elegido, escoge la figura y haz clic en el pentagrama. Arrastrar a la derecha pinta más notas, arrastrar una nota la mueve, soltarla sin moverla le pone la figura elegida y el clic derecho borra. Los silencios se ponen solos.

Plugins: carga CLAP, VST3 y VST2. No carga Audio Units ni los plugins internos de FL Studio (son de formato cerrado). Abre proyectos `.flp`.

La guía con imágenes está en [`docs/daw.md`](../docs/daw.md).

Normalmente se abre desde el menú de Beethoven Craft («Nueva canción»), que le pasa el idioma. Para compilarlo solo:

    cargo build --release -p beethoven_daw

El programa queda en `target/release/beethoven_daw`.

## Licencia

Software libre bajo la GNU General Public License, versión 3 o posterior (`LICENSE`), sin garantía. Los avisos de copyright están en el encabezado de cada archivo y en `THIRD-PARTY-NOTICES.txt`, junto con las licencias de las librerías de terceros.
