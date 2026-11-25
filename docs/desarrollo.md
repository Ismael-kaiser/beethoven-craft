# Compilar y desarrollar

Son dos proyectos de Cargo en el mismo repositorio:

| Carpeta | Qué es |
|---|---|
| `src/` | Beethoven Movie y el menú principal (egui) |
| `src/ui/` | Sus pestañas y ventanas |
| `crates/audio-fx` | Efectos e instrumentos de audio |
| `crates/craft-gpu` | Compositor de tarjeta gráfica (wgpu) de EffectCraft, más los efectos `bc:` y los de FilmCraft como núcleos (`shaders/bc.wgsl`) |
| `src/vgpu.rs` | Une la composición de Movie con la tarjeta; `composicion.rs` es la versión del procesador |
| `crates/craft-trans` | Transiciones y efectos de FilmCraft |
| `daw-demo/` | Beethoven DAW, con su propio `Cargo.toml` (compila en el mismo `target/`); ver su `README.md` |
| `modelos/` | Modelo para separar voces (67 MB) |
| `empaque/` | Cómo se arma la app en cada sistema |
| `tests/` | Pruebas de Movie |

## Compilar

```sh
cargo build --release                       # Movie y menú
(cd daw-demo && cargo build --release)      # DAW (deja target/release/beethoven-daw-demo)
```

## Pruebas

```sh
cargo test --release
VIDEO=/ruta/video.mp4 cargo test --release -- --ignored --test-threads 1   # con un video real; una de ellas abre la salida de audio en silencio
(cd daw-demo && cargo test --release --no-fail-fast)
```

`CAPTURAS=/carpeta` guarda imágenes de la interfaz al correr `tests/interfaz.rs`.

## Idiomas

- Movie: los textos están en español y `recursos/idioma/en.json` los pasa al inglés.
- DAW: igual, en español, y `daw-demo/recursos/en.json` los pasa al inglés.

## Contribuir

Corre las pruebas antes de mandar un cambio, conserva los avisos de copyright de cada archivo y, si agregas código o material de otros, ponlo en `AVISOS.md`. Lo que agregues al código propio queda bajo MIT o Apache-2.0; lo que toque las partes de Lemon Studio (ver `AVISOS.md`), bajo la GPL-3.0 o posterior.
