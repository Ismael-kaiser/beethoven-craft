# Avisos

El código propio de Beethoven Craft (creado por mixter-ha99) está bajo MIT (`LICENSE-MIT`) o Apache-2.0 (`LICENSE-APACHE`), a elección de quien lo use.

Las versiones hasta la 0.12.0 incluían piezas de Lemon Studio (GPL-3.0 o posterior) y siguen bajo esa licencia. Después de la 0.12.0 esas piezas se quitaron: los efectos de sonido, el sintetizador, los controles y los íconos son código nuevo de Beethoven Craft.

Beethoven DAW (`daw-demo/`) está escrito desde cero; sus avisos están en `daw-demo/AVISOS.md`. Trae el banco de instrumentos grabados MuseScore General (MIT; FluidR3 © Frank Wen y otros), con su licencia en `sonidos/MuseScore_General_License.md`.

Incluye trabajo de otras personas:

- **EffectCraft** (https://github.com/storytold/effectcraft, commit 1738360), Copyright (c) 2026 ArtCraft Team y colaboradores, MIT o Apache-2.0. Efectos de video, y su compositor de tarjeta gráfica (`crates/craft-gpu`, recortado el 2026-11-23; `mesa.rs` y `shaders/bc.wgsl` son de Beethoven Craft; sus licencias y su `NOTICE` van en esa carpeta).
- **FilmCraft** (https://github.com/storytold/filmcraft, commit 17221b5), del mismo equipo, MIT o Apache-2.0. Transiciones y 17 efectos de luz y movimiento (`crates/craft-trans`). Licencias en `recursos/`. Sus marcas no se usan: Beethoven Craft no está afiliado a ArtCraft, EffectCraft ni FilmCraft.
- **AniCraft** (https://github.com/storytold/anicraft, commit 7ce262d), Copyright (c) 2026 ArtCraft Team y colaboradores, MIT o Apache-2.0. Las curvas de animación (`src/curvas.rs`, de `crates/doc/src/easing.rs`).
- **Cuprum** (Jovanny Lemonad) y **Barlow** (The Barlow Project Authors), SIL Open Font License 1.1 (`recursos/Cuprum-OFL.txt`, `recursos/Barlow-OFL.txt`).
- **UVR-MDX-NET-Voc_FT**, modelo de Ultimate Vocal Remover (Anjok07, aufr33 y colaboradores), MIT con crédito. Se ejecuta en tu Mac con `tract`.
- Bibliotecas de Rust: ver `Cargo.lock` (egui, cpal, rfd, serde, realfft, tract, fontdue y sus dependencias).
- **ffmpeg** se usa como programa aparte. En Mac y Linux no viene incluido. En el paquete de Windows vienen `ffmpeg.exe` y `ffprobe.exe` sin cambios, de la compilación GPL de https://github.com/BtbN/FFmpeg-Builds (ahí están el código fuente y cómo se compila); su licencia es `FFmpeg-LICENSE.txt`. Más en https://ffmpeg.org.
