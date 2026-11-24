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
| `daw/` | Beethoven DAW, con su propio `Cargo.toml` |
| `daw/crates/os-app` | Interfaz del DAW; las pestañas propias son `tabs.rs`, `page.rs`, `score.rs`, `ideas.rs` y `grooves.rs` |
| `daw/crates/os-engine`, `os-dsp` | Motor de sonido |
| `daw/plugins` | Instrumentos y efectos incluidos |
| `modelos/` | Modelo para separar voces (67 MB) |
| `empaque/` | Cómo se arma la app en cada sistema |
| `tests/` | Pruebas de Movie |

## Compilar

```sh
cargo build --release                                   # Movie y menú
(cd daw && cargo build --release -p beethoven_daw)      # DAW
```

El DAW usa optimización entre crates (`lto = "thin"`): compilarlo desde cero tarda varios minutos.

## Pruebas

```sh
cargo test --release
VIDEO=/ruta/video.mp4 cargo test --release -- --ignored --test-threads 1   # con un video real; una de ellas abre la salida de audio en silencio
(cd daw && cargo test -p os-app -p os-ui -p os-theme -p os-session --lib)
```

`CAPTURAS=/carpeta` guarda imágenes de la interfaz al correr `tests/interfaz.rs`.

El DAW se puede ver sin abrir ventana:

```sh
cd daw
BEETHOVEN_IDIOMA=es target/release/beethoven_daw --screenshot salida.png --view score --demo --no-rack --no-browser --size 1500x900
```

`--view` acepta `playlist`, `rhythm`, `piano`, `mixer`, `score`, `ideas` y `export`.

## Idiomas

- Movie: los textos están en español y `recursos/idioma/en.json` los pasa al inglés.
- DAW: los textos están en inglés y `daw/crates/os-ui/es.tsv` los pasa al español (inglés, tabulador, español).

## Contribuir

Corre las pruebas antes de mandar un cambio, conserva los avisos de copyright de cada archivo y, si agregas código o material de otros, ponlo en `AVISOS.md`. Todo lo que se agregue debe ser compatible con la GPL-3.0.
