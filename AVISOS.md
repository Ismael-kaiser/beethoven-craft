# Avisos

Beethoven Craft (creado por mixter-ha99) es software libre bajo la GNU GPL, versión 3 o posterior (archivo `LICENSE`). Si compartes la app, tienes que compartir también este código bajo la misma licencia.

Incluye trabajo de otras personas:

- **Beethoven DAW** (carpeta `daw/`) es una versión modificada de **Lemon Studio** (https://github.com/ItsAlphaNeon/Lemon-Studio, commit 349c6ba), Copyright (C) 2026 ItsAlphaNeon y colaboradores, GPL-3.0-or-later. Cambios del 2026-11-23: se cambió el nombre del programa y de sus plugins de «Lemon» a «Beethoven» (los avisos de copyright y los enlaces al original se conservan; cada archivo dice «based on Lemon Studio»), los nombres que no caben se abrevian a «B.», y se apagaron el actualizador automático y la instalación de plugins al arrancar. Además se cambió la interfaz (pestañas, barra superior, colores, ritmo, mezcla y arreglo) y se agregaron las pestañas Partitura, Ideas, Ritmos listos y Exportar (`tabs.rs`, `score.rs`, `ideas.rs`, `grooves.rs`, `page.rs`, `views/page.rs`, `views/topbar.rs`). El `README.md` es de Beethoven Craft; `THIRD-PARTY-NOTICES.txt` sigue como lo dejó su autor.
- **Lemon Studio**, además: de ahí vienen los efectos de audio (`crates/audio-fx`, copiados el 2026-11-23 sin sus módulos de prueba) y los colores y controles de la piel «Lemon» (`src/ui/estilo.rs`, redibujados con egui). El autor de Lemon Studio pide, sin que la licencia obligue, que su código no se use en ArtCraft; Beethoven Craft no es ArtCraft, pero sí usa piezas de los dos.
- **EffectCraft** (https://github.com/storytold/effectcraft, commit 1738360), Copyright (c) 2026 ArtCraft Team y colaboradores, MIT o Apache-2.0. Efectos de video.
- **FilmCraft** (https://github.com/storytold/filmcraft, commit 17221b5), del mismo equipo, MIT o Apache-2.0. Transiciones y 17 efectos de luz y movimiento (`crates/craft-trans`). Licencias en `recursos/`. Sus marcas no se usan: Beethoven Craft no está afiliado a ArtCraft, EffectCraft, FilmCraft ni Lemon Studio.
- **AniCraft** (https://github.com/storytold/anicraft, commit 7ce262d), Copyright (c) 2026 ArtCraft Team y colaboradores, MIT o Apache-2.0. Las curvas de animación (`src/curvas.rs`, de `crates/doc/src/easing.rs`).
- **Íconos Tabler** 3.31.0, MIT, Copyright (c) 2020-2024 Paweł Kuna (`recursos/TABLER-LICENSE.txt`).
- **Cuprum** (Jovanny Lemonad) y **Barlow** (The Barlow Project Authors), SIL Open Font License 1.1 (`recursos/Cuprum-OFL.txt`, `recursos/Barlow-OFL.txt`).
- **UVR-MDX-NET-Voc_FT**, modelo de Ultimate Vocal Remover (Anjok07, aufr33 y colaboradores), MIT con crédito. Se ejecuta en tu Mac con `tract`.
- Bibliotecas de Rust: ver `Cargo.lock` (egui, cpal, rfd, serde, realfft, tract, fontdue y sus dependencias).
- **ffmpeg** se usa como programa aparte. En Mac y Linux no viene incluido. En el paquete de Windows vienen `ffmpeg.exe` y `ffprobe.exe` sin cambios, de la compilación GPL de https://github.com/BtbN/FFmpeg-Builds (ahí están el código fuente y cómo se compila); su licencia es `FFmpeg-LICENSE.txt`. Más en https://ffmpeg.org.
