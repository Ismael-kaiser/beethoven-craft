// Beethoven Craft. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Los efectos incluidos: sus nombres, controles y rangos, y cómo mueven el sonido de `sonido.rs` y `propios.rs`.
use crate::modelo::{SR, db};
use crate::propios;
use crate::sonido::{self, Banda, Bandas};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoFx {
    Ecualizador,
    Compresor,
    Compuerta,
    Limitador,
    Reverberacion,
    Eco,
    Coro,
    Faser,
    Saturador,
    Multibanda,
    Flanger,
    Filtro,
    RecorteSuave,
    Transitorios,
    Amplitud,
    Afinador,
    CambioTono,
    Seseo,
    Doblador,
    Excitador,
    Tremolo,
    Autopaneo,
    Wah,
    Robot,
    Bits,
    Karaoke,
    Armonias,
    Vibrato,
    Telefono,
    Vinilo,
    Tartamudeo,
    SubGrave,
    Parametrico,
    Distorsion,
    Moldeador,
    Vocoder,
    Balance,
    /// Un plugin de Beethoven DAW: sus controles van en `p`, en el orden de su receta, y lo demás en `Fx::plugin`.
    Plugin,
}

pub struct Param {
    pub nombre: &'static str,
    pub min: f32,
    pub max: f32,
    pub def: f32,
    pub unidad: &'static str,
    pub log: bool,
}

const fn p(nombre: &'static str, min: f32, max: f32, def: f32, unidad: &'static str, log: bool) -> Param {
    Param { nombre, min, max, def, unidad, log }
}

impl TipoFx {
    pub const TODOS: [TipoFx; 37] = [
        TipoFx::Ecualizador,
        TipoFx::Compresor,
        TipoFx::Compuerta,
        TipoFx::Limitador,
        TipoFx::Reverberacion,
        TipoFx::Eco,
        TipoFx::Coro,
        TipoFx::Faser,
        TipoFx::Saturador,
        TipoFx::Multibanda,
        TipoFx::Flanger,
        TipoFx::Filtro,
        TipoFx::RecorteSuave,
        TipoFx::Transitorios,
        TipoFx::Amplitud,
        TipoFx::Afinador,
        TipoFx::CambioTono,
        TipoFx::Seseo,
        TipoFx::Doblador,
        TipoFx::Excitador,
        TipoFx::Tremolo,
        TipoFx::Autopaneo,
        TipoFx::Wah,
        TipoFx::Robot,
        TipoFx::Bits,
        TipoFx::Karaoke,
        TipoFx::Armonias,
        TipoFx::Vibrato,
        TipoFx::Telefono,
        TipoFx::Vinilo,
        TipoFx::Tartamudeo,
        TipoFx::SubGrave,
        TipoFx::Parametrico,
        TipoFx::Distorsion,
        TipoFx::Moldeador,
        TipoFx::Vocoder,
        TipoFx::Balance,
    ];

    /// Grupo en la biblioteca.
    pub fn grupo(self) -> &'static str {
        match self {
            TipoFx::Afinador | TipoFx::CambioTono | TipoFx::Seseo | TipoFx::Doblador | TipoFx::Karaoke | TipoFx::Robot | TipoFx::Armonias => "Voz",
            TipoFx::Compresor | TipoFx::Compuerta | TipoFx::Limitador | TipoFx::Multibanda | TipoFx::Transitorios => "Dinámica",
            TipoFx::Ecualizador | TipoFx::Filtro | TipoFx::Wah | TipoFx::Excitador | TipoFx::SubGrave | TipoFx::Telefono => "Tono y filtros",
            TipoFx::Reverberacion | TipoFx::Eco | TipoFx::Amplitud | TipoFx::Autopaneo => "Espacio",
            TipoFx::Coro | TipoFx::Faser | TipoFx::Flanger | TipoFx::Tremolo | TipoFx::Vibrato | TipoFx::Tartamudeo => "Modulación",
            TipoFx::Saturador | TipoFx::RecorteSuave | TipoFx::Bits | TipoFx::Vinilo => "Distorsión",
            TipoFx::Parametrico | TipoFx::Distorsion | TipoFx::Moldeador | TipoFx::Vocoder | TipoFx::Balance => "Beethoven",
            TipoFx::Plugin => "Plugins de Beethoven DAW",
        }
    }
    pub const GRUPOS: [&'static str; 7] = ["Beethoven", "Voz", "Dinámica", "Tono y filtros", "Espacio", "Modulación", "Distorsión"];

    /// El grupo, en el idioma de la persona.
    pub fn grupo_ver(self) -> &'static str {
        crate::idioma::t(self.grupo())
    }

    pub fn nombre(self) -> &'static str {
        crate::idioma::t(self.nombre_es())
    }

    fn nombre_es(self) -> &'static str {
        match self {
            TipoFx::Ecualizador => "Ecualizador",
            TipoFx::Compresor => "Compresor",
            TipoFx::Compuerta => "Compuerta de ruido",
            TipoFx::Limitador => "Limitador",
            TipoFx::Reverberacion => "Reverberación",
            TipoFx::Eco => "Eco",
            TipoFx::Coro => "Coro",
            TipoFx::Faser => "Fáser",
            TipoFx::Saturador => "Saturador",
            TipoFx::Multibanda => "Compresión multibanda",
            TipoFx::Flanger => "Flanger",
            TipoFx::Filtro => "Filtro",
            TipoFx::RecorteSuave => "Recorte suave",
            TipoFx::Transitorios => "Transitorios",
            TipoFx::Amplitud => "Amplitud estéreo",
            TipoFx::Afinador => "Afinador de voz",
            TipoFx::CambioTono => "Cambio de tono",
            TipoFx::Seseo => "Quitar eses",
            TipoFx::Doblador => "Doblar voz",
            TipoFx::Excitador => "Brillo",
            TipoFx::Tremolo => "Trémolo",
            TipoFx::Autopaneo => "Paneo automático",
            TipoFx::Wah => "Wah",
            TipoFx::Robot => "Robot",
            TipoFx::Bits => "Triturador de bits",
            TipoFx::Karaoke => "Karaoke",
            TipoFx::Armonias => "Armonías",
            TipoFx::Vibrato => "Vibrato",
            TipoFx::Telefono => "Teléfono",
            TipoFx::Vinilo => "Vinilo",
            TipoFx::Tartamudeo => "Tartamudeo",
            TipoFx::SubGrave => "Graves profundos",
            TipoFx::Parametrico => "Ecualizador paramétrico",
            TipoFx::Distorsion => "Distorsión rápida",
            TipoFx::Moldeador => "Moldeador estéreo",
            TipoFx::Vocoder => "Vocoder",
            TipoFx::Balance => "Balance",
            TipoFx::Plugin => "Plugin de Beethoven DAW",
        }
    }

    pub fn ayuda(self) -> &'static str {
        crate::idioma::t(self.ayuda_es())
    }

    fn ayuda_es(self) -> &'static str {
        match self {
            TipoFx::Ecualizador => "Sube o baja graves, medios y agudos",
            TipoFx::Compresor => "Compresión de audio: empareja el volumen y da calidad",
            TipoFx::Compuerta => "Calla el ruido cuando no cantas",
            TipoFx::Limitador => "Evita que el sonido truene",
            TipoFx::Reverberacion => "Sensación de cuarto o sala",
            TipoFx::Eco => "Repeticiones",
            TipoFx::Coro => "Engorda el sonido",
            TipoFx::Faser => "Barrido giratorio",
            TipoFx::Saturador => "Calidez o distorsión",
            TipoFx::Multibanda => "Comprime graves, medios y agudos por separado: mejora la calidad final",
            TipoFx::Flanger => "Barrido tipo avión",
            TipoFx::Filtro => "Filtro con resonancia",
            TipoFx::RecorteSuave => "Redondea los picos",
            TipoFx::Transitorios => "Más o menos golpe en el ataque",
            TipoFx::Amplitud => "Abre o cierra el estéreo",
            TipoFx::Afinador => "Lleva tu voz a la nota más cercana de la escala",
            TipoFx::CambioTono => "Sube o baja semitonos",
            TipoFx::Seseo => "Baja el silbido de las eses",
            TipoFx::Doblador => "Como si cantaras dos veces",
            TipoFx::Excitador => "Agrega aire y claridad",
            TipoFx::Tremolo => "Sube y baja el volumen",
            TipoFx::Autopaneo => "Va de un oído al otro",
            TipoFx::Wah => "Filtro que habla",
            TipoFx::Robot => "Voz metálica",
            TipoFx::Bits => "Sonido de consola vieja",
            TipoFx::Karaoke => "Quita lo que está al centro, casi siempre la voz",
            TipoFx::Armonias => "Te hace coros con tu propia voz, dentro de la escala",
            TipoFx::Vibrato => "Hace temblar el tono",
            TipoFx::Telefono => "Suena como por teléfono o radio chico",
            TipoFx::Vinilo => "Disco viejo: bamboleo, siseo y chasquidos",
            TipoFx::Tartamudeo => "Corta el sonido a tiempo con el tempo",
            TipoFx::SubGrave => "Agrega una octava abajo y armónicos para que se sientan los bajos",
            TipoFx::Parametrico => "Siete bandas que se mueven con el ratón sobre la curva",
            TipoFx::Distorsion => "Distorsión con dos carácteres, de tibia a rota",
            TipoFx::Moldeador => "Mezcla, retrasa y gira la fase de cada lado del estéreo",
            TipoFx::Vocoder => "Tu voz habla por un sintetizador",
            TipoFx::Balance => "Balance entre izquierda y derecha, y volumen",
            TipoFx::Plugin => "Un efecto de Beethoven DAW, con su ventana",
        }
    }

    pub fn params(self) -> &'static [Param] {
        match self {
            TipoFx::Ecualizador => {
                const P: &[Param] = &[
                    p("Corte de graves", 20.0, 400.0, 20.0, "Hz", true),
                    p("Graves (120 Hz)", -18.0, 18.0, 0.0, "dB", false),
                    p("Medios", -18.0, 18.0, 0.0, "dB", false),
                    p("Centro de medios", 200.0, 4000.0, 700.0, "Hz", true),
                    p("Presencia (4 kHz)", -18.0, 18.0, 0.0, "dB", false),
                    p("Agudos (9 kHz)", -18.0, 18.0, 0.0, "dB", false),
                    p("Corte de agudos", 2000.0, 20000.0, 20000.0, "Hz", true),
                    p("Salida", -18.0, 18.0, 0.0, "dB", false),
                ];
                P
            }
            TipoFx::Compresor => {
                const P: &[Param] = &[
                    p("Umbral", -50.0, 0.0, -20.0, "dB", false),
                    p("Proporción", 1.0, 20.0, 3.0, ":1", true),
                    p("Ataque", 0.1, 200.0, 8.0, "ms", true),
                    p("Liberación", 5.0, 900.0, 120.0, "ms", true),
                    p("Ganancia", -12.0, 24.0, 0.0, "dB", false),
                ];
                P
            }
            TipoFx::Compuerta => {
                const P: &[Param] = &[
                    p("Umbral", -80.0, -10.0, -45.0, "dB", false),
                    p("Atenuación", -80.0, 0.0, -60.0, "dB", false),
                    p("Liberación", 5.0, 900.0, 80.0, "ms", true),
                ];
                P
            }
            TipoFx::Limitador => {
                const P: &[Param] = &[
                    p("Entrada", 0.0, 16.0, 0.0, "dB", false),
                    p("Techo", -12.0, 0.0, -1.0, "dB", false),
                    p("Liberación", 5.0, 900.0, 60.0, "ms", true),
                ];
                P
            }
            TipoFx::Reverberacion => {
                const P: &[Param] = &[
                    p("Mezcla", 0.0, 100.0, 45.0, "%", false),
                    p("Duración", 0.2, 15.0, 1.8, "s", true),
                    p("Tamaño", 1.0, 100.0, 55.0, "%", false),
                    p("Retraso previo", 0.0, 200.0, 20.0, "ms", false),
                    p("Corte de graves", 20.0, 1000.0, 150.0, "Hz", true),
                    p("Brillo", 500.0, 22000.0, 8000.0, "Hz", true),
                ];
                P
            }
            TipoFx::Eco => {
                const P: &[Param] = &[
                    p("Tiempo", 20.0, 1000.0, 350.0, "ms", true),
                    p("Repetición", 0.0, 95.0, 35.0, "%", false),
                    p("Mezcla", 0.0, 100.0, 25.0, "%", false),
                    p("Brillo", 300.0, 20000.0, 5000.0, "Hz", true),
                    p("Rebote estéreo", 0.0, 1.0, 0.0, "", false),
                ];
                P
            }
            TipoFx::Coro => {
                const P: &[Param] = &[
                    p("Velocidad", 0.05, 5.0, 0.6, "Hz", true),
                    p("Profundidad", 0.0, 100.0, 45.0, "%", false),
                    p("Retraso", 0.1, 30.0, 12.0, "ms", false),
                    p("Estéreo", 0.0, 100.0, 60.0, "%", false),
                    p("Mezcla", 0.0, 100.0, 50.0, "%", false),
                ];
                P
            }
            TipoFx::Faser => {
                const P: &[Param] = &[
                    p("Velocidad", 0.02, 2.0, 0.3, "Hz", true),
                    p("Profundidad", 0.0, 100.0, 80.0, "%", false),
                    p("Realimentación", 0.0, 95.0, 40.0, "%", false),
                    p("Mezcla", 0.0, 100.0, 50.0, "%", false),
                ];
                P
            }
            TipoFx::Saturador => {
                const P: &[Param] = &[
                    p("Empuje", 0.0, 100.0, 30.0, "%", false),
                    p("Mezcla", 0.0, 100.0, 60.0, "%", false),
                    p("Salida", -24.0, 6.0, -3.0, "dB", false),
                ];
                P
            }
            TipoFx::Multibanda => {
                const P: &[Param] = &[
                    p("Cruce grave", 60.0, 600.0, 200.0, "Hz", true),
                    p("Cruce agudo", 1000.0, 10000.0, 3000.0, "Hz", true),
                    p("Empuje graves", -12.0, 12.0, 2.0, "dB", false),
                    p("Empuje medios", -12.0, 12.0, 2.0, "dB", false),
                    p("Empuje agudos", -12.0, 12.0, 2.0, "dB", false),
                    p("Empuje final", -12.0, 12.0, 0.0, "dB", false),
                    p("Liberación", 5.0, 900.0, 120.0, "ms", true),
                    p("Salida", -18.0, 0.0, -1.0, "dB", false),
                ];
                P
            }
            TipoFx::Flanger => {
                const P: &[Param] = &[
                    p("Velocidad", 0.02, 5.0, 0.3, "Hz", true),
                    p("Profundidad", 0.1, 15.0, 4.0, "ms", true),
                    p("Retraso", 0.0, 15.0, 0.5, "ms", false),
                    p("Realimentación", 0.0, 95.0, 40.0, "%", false),
                    p("Mezcla", 0.0, 100.0, 50.0, "%", false),
                ];
                P
            }
            TipoFx::Filtro => {
                const P: &[Param] = &[
                    p("Corte", 30.0, 15000.0, 6000.0, "Hz", true),
                    p("Resonancia", 0.0, 100.0, 20.0, "%", false),
                    p("Pasa graves", 0.0, 100.0, 100.0, "%", false),
                    p("Pasa banda", 0.0, 100.0, 0.0, "%", false),
                    p("Pasa agudos", 0.0, 100.0, 0.0, "%", false),
                ];
                P
            }
            TipoFx::RecorteSuave => {
                const P: &[Param] = &[p("Umbral", 1.0, 100.0, 70.0, "%", false), p("Salida", -24.0, 3.0, 0.0, "dB", false)];
                P
            }
            TipoFx::Transitorios => {
                const P: &[Param] = &[
                    p("Ataque", -100.0, 100.0, 30.0, "%", false),
                    p("Cola", -100.0, 100.0, 0.0, "%", false),
                    p("Salida", -24.0, 6.0, 0.0, "dB", false),
                ];
                P
            }
            TipoFx::Amplitud => {
                const P: &[Param] = &[p("Apertura", -100.0, 100.0, 30.0, "%", false), p("Salida", -24.0, 6.0, 0.0, "dB", false)];
                P
            }
            TipoFx::Afinador => {
                const P: &[Param] = &[
                    p("Tono", 0.0, 11.0, 0.0, "nota", false),
                    p("Escala", 0.0, 4.0, 1.0, "esc", false),
                    p("Rapidez", 1.0, 400.0, 25.0, "ms", true),
                    p("Cantidad", 0.0, 100.0, 100.0, "%", false),
                    p("Transportar", -12.0, 12.0, 0.0, "st", false),
                ];
                P
            }
            TipoFx::CambioTono => {
                const P: &[Param] = &[p("Semitonos", -12.0, 12.0, 0.0, "st", false), p("Fino", -100.0, 100.0, 0.0, "cents", false), p("Mezcla", 0.0, 100.0, 100.0, "%", false)];
                P
            }
            TipoFx::Seseo => {
                const P: &[Param] = &[p("Desde", 2000.0, 12000.0, 5500.0, "Hz", true), p("Umbral", -60.0, 0.0, -30.0, "dB", false), p("Máximo", 0.0, 30.0, 12.0, "dB", false)];
                P
            }
            TipoFx::Doblador => {
                const P: &[Param] = &[p("Desafinar", 0.0, 50.0, 12.0, "cents", false), p("Retraso", 1.0, 70.0, 18.0, "ms", false), p("Mezcla", 0.0, 100.0, 60.0, "%", false)];
                P
            }
            TipoFx::Excitador => {
                const P: &[Param] = &[p("Desde", 1000.0, 10000.0, 3500.0, "Hz", true), p("Empuje", 0.0, 100.0, 40.0, "%", false), p("Mezcla", 0.0, 100.0, 50.0, "%", false)];
                P
            }
            TipoFx::Tremolo | TipoFx::Autopaneo => {
                const P: &[Param] = &[p("Velocidad", 0.05, 20.0, 4.0, "Hz", true), p("Profundidad", 0.0, 100.0, 70.0, "%", false), p("Forma", 0.0, 100.0, 0.0, "%", false)];
                P
            }
            TipoFx::Wah => {
                const P: &[Param] = &[
                    p("Base", 100.0, 1500.0, 350.0, "Hz", true),
                    p("Vaivén", 0.1, 10.0, 1.5, "Hz", true),
                    p("Por fuerza", 0.0, 100.0, 30.0, "%", false),
                    p("Por vaivén", 0.0, 100.0, 70.0, "%", false),
                    p("Mezcla", 0.0, 100.0, 80.0, "%", false),
                ];
                P
            }
            TipoFx::Robot => {
                const P: &[Param] = &[p("Frecuencia", 10.0, 2000.0, 75.0, "Hz", true), p("Mezcla", 0.0, 100.0, 85.0, "%", false)];
                P
            }
            TipoFx::Bits => {
                const P: &[Param] = &[p("Bits", 2.0, 16.0, 6.0, "bits", false), p("Muestreo", 1000.0, 48000.0, 8000.0, "Hz", true), p("Mezcla", 0.0, 100.0, 100.0, "%", false)];
                P
            }
            TipoFx::Armonias => {
                const P: &[Param] = &[
                    p("Raíz", 0.0, 11.0, 0.0, "nota", false),
                    p("Escala", 0.0, 4.0, 1.0, "esc", false),
                    p("Voz 1 (grados)", -7.0, 7.0, 2.0, "gr", false),
                    p("Voz 2 (grados)", -7.0, 7.0, 4.0, "gr", false),
                    p("Volumen 1", 0.0, 100.0, 60.0, "%", false),
                    p("Volumen 2", 0.0, 100.0, 45.0, "%", false),
                    p("Abrir", 0.0, 100.0, 70.0, "%", false),
                ];
                P
            }
            TipoFx::Vibrato => {
                const P: &[Param] = &[p("Velocidad", 0.1, 14.0, 5.5, "Hz", true), p("Profundidad", 0.0, 100.0, 35.0, "cents", false)];
                P
            }
            TipoFx::Telefono => {
                const P: &[Param] = &[p("Desde", 100.0, 1000.0, 400.0, "Hz", true), p("Hasta", 1500.0, 8000.0, 3200.0, "Hz", true), p("Rasposo", 0.0, 100.0, 45.0, "%", false), p("Mezcla", 0.0, 100.0, 100.0, "%", false)];
                P
            }
            TipoFx::Vinilo => {
                const P: &[Param] = &[p("Bamboleo", 0.0, 100.0, 40.0, "%", false), p("Siseo", 0.0, 100.0, 25.0, "%", false), p("Chasquidos", 0.0, 100.0, 35.0, "%", false), p("Opaco", 0.0, 100.0, 50.0, "%", false)];
                P
            }
            TipoFx::Tartamudeo => {
                const P: &[Param] = &[p("Tempo", 40.0, 240.0, 120.0, "bpm", false), p("Cortes por golpe", 1.0, 8.0, 4.0, "", false), p("Abierto", 5.0, 100.0, 50.0, "%", false), p("Suavidad", 0.5, 30.0, 4.0, "ms", true), p("Profundidad", 0.0, 100.0, 100.0, "%", false)];
                P
            }
            TipoFx::Parametrico => {
                macro_rules! banda {
                    ($hz:expr, $tipo:expr) => {
                        [p("Tipo", 0.0, 7.0, $tipo, "banda", false), p("Frecuencia", 20.0, 20000.0, $hz, "Hz", true), p("Nivel", -18.0, 18.0, 0.0, "dB", false), p("Ancho", 1.0, 100.0, 39.0, "%", false)]
                    };
                }
                const B: [[Param; 4]; 7] = [banda!(60.0, 5.0), banda!(150.0, 6.0), banda!(400.0, 6.0), banda!(1000.0, 6.0), banda!(2500.0, 6.0), banda!(6000.0, 6.0), banda!(12000.0, 7.0)];
                const fn todas() -> [Param; 29] {
                    let mut v = [const { p("Salida", -18.0, 18.0, 0.0, "dB", false) }; 29];
                    let mut i = 0;
                    while i < 28 {
                        let x = &B[i / 4][i % 4];
                        v[i] = p(x.nombre, x.min, x.max, x.def, x.unidad, x.log);
                        i += 1;
                    }
                    v
                }
                static P: [Param; 29] = todas();
                &P
            }
            TipoFx::Distorsion => {
                const P: &[Param] = &[
                    p("Entrada", -100.0, 100.0, 40.0, "%", false),
                    p("Umbral", 1.0, 10.0, 5.0, "", false),
                    p("Carácter", 0.0, 1.0, 0.0, "", false),
                    p("Mezcla", 0.0, 100.0, 80.0, "%", false),
                    p("Salida", 0.0, 100.0, 70.0, "%", false),
                ];
                P
            }
            TipoFx::Moldeador => {
                const P: &[Param] = &[
                    p("Derecha a izquierda", -100.0, 100.0, 0.0, "%", false),
                    p("Izquierda", -100.0, 100.0, 100.0, "%", false),
                    p("Derecha", -100.0, 100.0, 100.0, "%", false),
                    p("Izquierda a derecha", -100.0, 100.0, 0.0, "%", false),
                    p("Retraso", -100.0, 100.0, 0.0, "%", false),
                    p("Fase", -100.0, 100.0, 0.0, "%", false),
                ];
                P
            }
            TipoFx::Vocoder => {
                const P: &[Param] = &[
                    p("Nota", 24.0, 72.0, 45.0, "midi", false),
                    p("Acorde", 0.0, 4.0, 2.0, "acorde", false),
                    p("Bandas", 8.0, 64.0, 32.0, "n", false),
                    p("Ataque", 0.5, 50.0, 2.0, "ms", true),
                    p("Liberación", 5.0, 400.0, 45.0, "ms", true),
                    p("Aire", 0.0, 100.0, 20.0, "%", false),
                    p("Mezcla", 0.0, 100.0, 100.0, "%", false),
                ];
                P
            }
            TipoFx::Plugin => &[],
            TipoFx::Balance => {
                const P: &[Param] = &[p("Balance", -100.0, 100.0, 0.0, "%", false), p("Volumen", 0.0, 125.0, 100.0, "%", false)];
                P
            }
            TipoFx::SubGrave => {
                const P: &[Param] = &[p("Hasta", 50.0, 250.0, 110.0, "Hz", true), p("Octava abajo", 0.0, 100.0, 45.0, "%", false), p("Armónicos", 0.0, 100.0, 35.0, "%", false)];
                P
            }
            TipoFx::Karaoke => {
                const P: &[Param] = &[p("Desde", 60.0, 500.0, 140.0, "Hz", true), p("Hasta", 2000.0, 16000.0, 9000.0, "Hz", true), p("Cantidad", 0.0, 100.0, 100.0, "%", false)];
                P
            }
        }
    }

    pub fn defaults(self) -> Vec<f32> {
        self.params().iter().map(|p| p.def).collect()
    }

    /// Con qué valores se pone un efecto nuevo: que se oiga desde el principio. `defaults` es el punto neutro.
    pub fn arranque(self) -> Vec<f32> {
        match self {
            TipoFx::Ecualizador => vec![60.0, 4.0, -3.0, 500.0, 3.0, 4.0, 20000.0, 0.0],
            TipoFx::Compresor => vec![-24.0, 4.0, 8.0, 120.0, 6.0],
            TipoFx::Compuerta => vec![-32.0, -60.0, 80.0],
            TipoFx::Limitador => vec![6.0, -1.0, 60.0],
            TipoFx::Filtro => vec![1500.0, 30.0, 100.0, 0.0, 0.0],
            TipoFx::RecorteSuave => vec![25.0, 0.0],
            TipoFx::Transitorios => vec![90.0, 40.0, 0.0],
            TipoFx::Amplitud => vec![70.0, 0.0],
            TipoFx::Eco => vec![350.0, 40.0, 40.0, 6000.0, 0.0],
            TipoFx::CambioTono => vec![3.0, 0.0, 100.0],
            TipoFx::Parametrico => {
                let mut v = self.defaults();
                (v[2], v[10], v[26]) = (3.0, -4.0, 4.0);
                v
            }
            TipoFx::Balance => vec![30.0, 100.0],
            TipoFx::Moldeador => vec![0.0, 100.0, 100.0, 0.0, 20.0, 0.0],
            _ => self.defaults(),
        }
    }

    /// Puntos de partida con nombre.
    pub fn ajustes(self) -> &'static [(&'static str, &'static [f32])] {
        match self {
            TipoFx::Ecualizador => &[
                ("Voz clara", &[90.0, -2.0, -2.5, 350.0, 3.0, 2.5, 20000.0, 0.0]),
                ("Dejar lugar a la voz", &[20.0, 0.0, -4.0, 1500.0, -3.0, 0.0, 20000.0, 0.0]),
                ("Teléfono", &[400.0, -12.0, 8.0, 1500.0, 0.0, -12.0, 3400.0, -3.0]),
            ],
            TipoFx::Compresor => &[
                ("Voz suave", &[-22.0, 2.5, 10.0, 150.0, 3.0]),
                ("Voz al frente", &[-28.0, 5.0, 4.0, 90.0, 7.0]),
                ("Mejorar calidad (parejo)", &[-24.0, 1.8, 20.0, 250.0, 3.5]),
            ],
            TipoFx::Vocoder => &[
                ("Robot clásico", &[45.0, 0.0, 24.0, 2.0, 40.0, 15.0, 100.0]),
                ("Coro de máquinas", &[48.0, 2.0, 40.0, 5.0, 90.0, 25.0, 100.0]),
                ("Oscuro", &[33.0, 3.0, 20.0, 4.0, 120.0, 10.0, 85.0]),
            ],
            TipoFx::Moldeador => &[
                ("A mono", &[50.0, 50.0, 50.0, 50.0, 0.0, 0.0]),
                ("Cambiar lados", &[100.0, 0.0, 0.0, 100.0, 0.0, 0.0]),
                ("Solo los lados", &[-50.0, 50.0, 50.0, -50.0, 0.0, 0.0]),
            ],
            TipoFx::Distorsion => &[
                ("Tibia", &[15.0, 8.0, 0.0, 50.0, 80.0]),
                ("Rota", &[80.0, 2.0, 1.0, 100.0, 50.0]),
            ],
            TipoFx::Reverberacion => &[
                ("Cuarto chico", &[30.0, 0.5, 20.0, 5.0, 200.0, 7000.0]),
                ("Sala", &[45.0, 1.8, 55.0, 20.0, 150.0, 8000.0]),
                ("Catedral", &[65.0, 6.0, 100.0, 50.0, 200.0, 5500.0]),
            ],
            TipoFx::Eco => &[
                ("Eco corto", &[110.0, 15.0, 30.0, 6000.0, 0.0]),
                ("Eco de balada", &[420.0, 35.0, 22.0, 4000.0, 1.0]),
            ],
            _ => &[],
        }
    }
}

pub trait Procesador: Send {
    /// Estéreo intercalado, en el lugar.
    fn procesar(&mut self, buf: &mut [f32]);
    fn ajustar(&mut self, p: &[f32]);
    fn limpiar(&mut self);
    fn latencia(&self) -> usize {
        0
    }
}

pub fn crear(tipo: TipoFx, p: &[f32]) -> Box<dyn Procesador> {
    let mut e: Box<dyn Procesador> = match tipo {
        TipoFx::Ecualizador => Box::new(Eq(Bandas::nuevas(6))),
        TipoFx::Parametrico => Box::new(Par(Bandas::nuevas(7))),
        TipoFx::Compresor => Box::new(sonido::Compresor::nuevo()),
        TipoFx::Compuerta => Box::new(sonido::Compuerta::nueva()),
        TipoFx::Limitador => Box::new(sonido::Limitador::nuevo()),
        TipoFx::Multibanda => Box::new(sonido::Multibanda::nueva()),
        TipoFx::Transitorios => Box::<sonido::Transitorios>::default(),
        TipoFx::Reverberacion => Box::new(sonido::Sala::nueva()),
        TipoFx::Eco => Box::new(sonido::Eco::nuevo()),
        TipoFx::Coro => Box::new(Coro { c: sonido::Coro::nuevo(), mezcla: 0.5 }),
        TipoFx::Flanger => Box::new(Flanger { f: sonido::Flanger::nuevo(), mezcla: 0.5 }),
        TipoFx::Faser => Box::<sonido::Faser>::default(),
        TipoFx::Filtro => Box::new(Filtro { f: Default::default(), p: [0.0; 3] }),
        TipoFx::Saturador | TipoFx::RecorteSuave | TipoFx::Distorsion | TipoFx::Amplitud | TipoFx::Balance => Box::new(Directo { tipo, p: vec![] }),
        // Sin su plugin (lo pone `motor.rs` con `plugins::crear`): pasa el sonido igual.
        TipoFx::Plugin => Box::new(Paso),
        TipoFx::Moldeador => Box::new(sonido::Moldeador::nuevo()),
        TipoFx::Vocoder => Box::new(Voc::nuevo()),
        TipoFx::Afinador => Box::new(propios::Afinador::nuevo()),
        TipoFx::CambioTono => Box::new(propios::CambioTono::nuevo()),
        TipoFx::Seseo => Box::<propios::Seseo>::default(),
        TipoFx::Doblador => Box::new(propios::Doblador::nuevo()),
        TipoFx::Excitador => Box::<propios::Excitador>::default(),
        TipoFx::Tremolo => Box::new(propios::Onda::nueva(false)),
        TipoFx::Autopaneo => Box::new(propios::Onda::nueva(true)),
        TipoFx::Wah => Box::<propios::Wah>::default(),
        TipoFx::Robot => Box::<propios::Anillo>::default(),
        TipoFx::Bits => Box::<propios::Bits>::default(),
        TipoFx::Karaoke => Box::<propios::Karaoke>::default(),
        TipoFx::Armonias => Box::new(propios::Armonias::nuevo()),
        TipoFx::Vibrato => Box::new(propios::Vibrato::nuevo()),
        TipoFx::Telefono => Box::<propios::Telefono>::default(),
        TipoFx::Vinilo => Box::new(propios::Vinilo::nuevo()),
        TipoFx::Tartamudeo => Box::<propios::Tartamudeo>::default(),
        TipoFx::SubGrave => Box::<propios::SubGrave>::default(),
    };
    e.ajustar(p);
    e.limpiar();
    e
}

struct Paso;

impl Procesador for Paso {
    fn procesar(&mut self, _: &mut [f32]) {}
    fn ajustar(&mut self, _: &[f32]) {}
    fn limpiar(&mut self) {}
}

/// `1 - mezcla / 2`: al mezclar copias del mismo sonido, lo seco baja menos.
fn seco(mezcla: f32) -> f32 {
    1.0 - mezcla * 0.5
}

// ---- Ecualizadores

struct Eq(Bandas);

impl Eq {
    fn poner(b: &mut Bandas, p: &[f32]) {
        let q = std::f32::consts::FRAC_1_SQRT_2;
        b.poner(0, if p[0] > 20.5 { Banda::PasaAgudos } else { Banda::Apagada }, p[0], 0.0, q);
        b.poner(1, Banda::MesetaGrave, 120.0, p[1], q);
        b.poner(2, Banda::Campana, p[3], p[2], 0.9);
        b.poner(3, Banda::Campana, 4000.0, p[4], 0.9);
        b.poner(4, Banda::MesetaAguda, 9000.0, p[5], q);
        b.poner(5, if p[6] < 19_900.0 { Banda::PasaGraves } else { Banda::Apagada }, p[6], 0.0, q);
        b.salida(p[7]);
    }
}

/// Curva del ecualizador en dB, para dibujarla.
pub fn curva_eq(p: &[f32], hz: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut b = Bandas::nuevas(6);
    Eq::poner(&mut b, p);
    b.curva(hz)
}

impl Procesador for Eq {
    fn procesar(&mut self, buf: &mut [f32]) {
        self.0.procesar(buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        Eq::poner(&mut self.0, p);
    }
    fn limpiar(&mut self) {
        self.0.limpiar();
    }
}

/// Nombres de los tipos de banda, en el orden de sus valores guardados.
pub const TIPOS_BANDA: [&str; 8] = ["Apagada", "Pasa graves", "Pasa banda", "Pasa agudos", "Quita banda", "Meseta grave", "Campana", "Meseta aguda"];
pub const ACORDES: [&str; 5] = ["Una nota", "Quinta", "Mayor", "Menor", "Séptima"];

struct Par(Bandas);

impl Par {
    fn poner(b: &mut Bandas, p: &[f32]) {
        use Banda::*;
        for k in 0..7 {
            let v = &p[k * 4..k * 4 + 4];
            let tipo = [Apagada, PasaGraves, PasaBanda, PasaAgudos, QuitaBanda, MesetaGrave, Campana, MesetaAguda][(v[0].round() as usize).min(7)];
            let ancho = (v[3] / 100.0).clamp(0.01, 1.0);
            let q = match tipo {
                // En octavas, de una esquina a la otra.
                Campana | PasaBanda | QuitaBanda => {
                    let o = 0.08 + ancho * 3.4;
                    1.0 / (2.0 * (std::f32::consts::LN_2 / 2.0 * o).sinh())
                }
                MesetaGrave | MesetaAguda => (1.1 - ancho).clamp(0.3, 1.0),
                // Angosto: con un pico en el corte.
                _ => (0.276 / ancho).clamp(0.4, 8.0),
            };
            b.poner(k, tipo, v[1], v[2], q);
        }
        b.salida(p[28]);
    }
}

/// Curva del paramétrico en dB.
pub fn curva_par(p: &[f32], hz: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut b = Bandas::nuevas(7);
    Par::poner(&mut b, p);
    b.curva(hz)
}

impl Procesador for Par {
    fn procesar(&mut self, buf: &mut [f32]) {
        self.0.procesar(buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        Par::poner(&mut self.0, p);
    }
    fn limpiar(&mut self) {
        self.0.limpiar();
    }
}

// ---- Dinámica

impl Procesador for sonido::Compresor {
    fn procesar(&mut self, buf: &mut [f32]) {
        sonido::Compresor::procesar(self, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        (self.umbral, self.razon, self.ataque, self.suelta, self.ganancia) = (p[0], p[1], p[2], p[3], db(p[4]));
    }
    fn limpiar(&mut self) {
        sonido::Compresor::limpiar(self);
    }
}

impl Procesador for sonido::Compuerta {
    fn procesar(&mut self, buf: &mut [f32]) {
        sonido::Compuerta::procesar(self, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        (self.umbral, self.cerrada, self.suelta) = (db(p[0]), db(p[1]), p[2]);
    }
    fn limpiar(&mut self) {
        sonido::Compuerta::limpiar(self);
    }
}

impl Procesador for sonido::Limitador {
    fn procesar(&mut self, buf: &mut [f32]) {
        sonido::Limitador::procesar(self, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        (self.entrada, self.techo, self.suelta) = (db(p[0]), db(p[1]), p[2]);
    }
    fn limpiar(&mut self) {
        sonido::Limitador::limpiar(self);
    }
    fn latencia(&self) -> usize {
        sonido::Limitador::latencia()
    }
}

impl Procesador for sonido::Multibanda {
    fn procesar(&mut self, buf: &mut [f32]) {
        sonido::Multibanda::procesar(self, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        sonido::Multibanda::ajustar(self, [p[0], p[1]], [p[2], p[3], p[4], p[5]], p[6], p[7]);
    }
    fn limpiar(&mut self) {
        sonido::Multibanda::limpiar(self);
    }
    fn latencia(&self) -> usize {
        sonido::Limitador::latencia()
    }
}

impl Procesador for sonido::Transitorios {
    fn procesar(&mut self, buf: &mut [f32]) {
        sonido::Transitorios::procesar(self, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        (self.golpe, self.cola, self.salida) = (p[0] / 100.0, p[1] / 100.0, db(p[2]));
    }
    fn limpiar(&mut self) {
        sonido::Transitorios::limpiar(self);
    }
}

// ---- Espacio

impl Procesador for sonido::Sala {
    fn procesar(&mut self, buf: &mut [f32]) {
        sonido::Sala::procesar(self, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        sonido::Sala::ajustar(self, p[0] / 100.0, p[1], p[2] / 100.0, p[3], p[4], p[5]);
    }
    fn limpiar(&mut self) {
        sonido::Sala::limpiar(self);
    }
}

impl Procesador for sonido::Eco {
    fn procesar(&mut self, buf: &mut [f32]) {
        sonido::Eco::procesar(self, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        (self.ms, self.repite, self.mezcla, self.hz, self.rebote) = (p[0], p[1] / 100.0, p[2] / 100.0, p[3], p[4] >= 0.5);
    }
    fn limpiar(&mut self) {
        sonido::Eco::limpiar(self);
    }
}

// ---- Modulación

struct Coro {
    c: sonido::Coro,
    mezcla: f32,
}

impl Procesador for Coro {
    fn procesar(&mut self, buf: &mut [f32]) {
        self.c.procesar(buf, seco(self.mezcla), self.mezcla);
    }
    fn ajustar(&mut self, p: &[f32]) {
        (self.c.hz, self.c.hondo, self.c.ms, self.c.abierto, self.mezcla) = (p[0], p[1] / 100.0, p[2], p[3] / 100.0, p[4] / 100.0);
    }
    fn limpiar(&mut self) {
        self.c.limpiar();
    }
}

struct Flanger {
    f: sonido::Flanger,
    mezcla: f32,
}

impl Procesador for Flanger {
    fn procesar(&mut self, buf: &mut [f32]) {
        self.f.procesar(buf, seco(self.mezcla) * 0.7, self.mezcla * 0.7);
    }
    fn ajustar(&mut self, p: &[f32]) {
        (self.f.hz, self.f.hondo, self.f.ms, self.f.repite, self.mezcla) = (p[0], p[1], p[2], p[3] / 100.0, p[4] / 100.0);
    }
    fn limpiar(&mut self) {
        self.f.limpiar();
    }
}

impl Procesador for sonido::Faser {
    fn procesar(&mut self, buf: &mut [f32]) {
        sonido::Faser::procesar(self, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        (self.hz, self.hondo, self.repite, self.mezcla) = (p[0], p[1] / 100.0, p[2] / 100.0, p[3] / 100.0);
    }
    fn limpiar(&mut self) {
        sonido::Faser::limpiar(self);
    }
}

struct Filtro {
    f: sonido::Variable,
    /// Cuánto de graves, banda y agudos.
    p: [f32; 3],
}

impl Procesador for Filtro {
    fn procesar(&mut self, buf: &mut [f32]) {
        for f in buf.chunks_exact_mut(2) {
            for (k, x) in f.iter_mut().enumerate() {
                let y = self.f.paso(k, *x);
                *x = y[0] * self.p[0] + y[1] * self.p[1] + y[2] * self.p[2];
            }
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.f.poner(p[0], p[1] / 100.0);
        self.p = [p[2] / 100.0, p[3] / 100.0, p[4] / 100.0];
    }
    fn limpiar(&mut self) {
        self.f.limpiar();
    }
}

// ---- Los que no guardan nada de una muestra a la otra

struct Directo {
    tipo: TipoFx,
    p: Vec<f32>,
}

impl Procesador for Directo {
    fn procesar(&mut self, buf: &mut [f32]) {
        let p = &self.p;
        match self.tipo {
            TipoFx::Saturador => {
                let (empuje, m, g) = (1.0 + p[0] * 0.09, p[1] / 100.0, db(p[2]));
                let norma = empuje.powf(-0.28);
                buf.iter_mut().for_each(|x| *x = (*x * (1.0 - m) + (*x * empuje).tanh() * norma * m) * g);
            }
            TipoFx::RecorteSuave => {
                let (u, g) = (p[0] / 100.0, db(p[1]));
                buf.iter_mut().for_each(|x| *x = sonido::redondear(*x, u) * g);
            }
            TipoFx::Distorsion => {
                // Menos umbral, más roto; y que el nivel no se dispare con el empuje.
                let empuje = db(p[0] * 0.18) * 10.0 / p[1].max(1.0);
                let (norma, rota, m, g) = (empuje.max(1.0).powf(-0.75) * 0.75, p[2] >= 0.5, p[3] / 100.0, p[4] / 100.0);
                buf.iter_mut().for_each(|x| *x = (*x * (1.0 - m) + sonido::distorsion(*x, empuje, rota) * norma * m) * g);
            }
            TipoFx::Amplitud => {
                let (a, g) = (p[0] / 100.0, db(p[1]));
                // Al abrir, el centro baja un poco: así se oye más ancho y no más fuerte.
                let (lados, centro) = (1.0 + a, 1.0 - a.max(0.0) * 0.4);
                for f in buf.chunks_exact_mut(2) {
                    let (c, l) = ((f[0] + f[1]) * 0.5 * centro, (f[0] - f[1]) * 0.5 * lados);
                    (f[0], f[1]) = ((c + l) * g, (c - l) * g);
                }
            }
            _ => {
                let (b, g) = (p[0] / 100.0, p[1] / 100.0);
                // De igual potencia: mover a un lado no cambia cuánto se oye.
                let (s, c) = ((b + 1.0) * std::f32::consts::FRAC_PI_4).sin_cos();
                let (gi, gd) = (c * std::f32::consts::SQRT_2 * g, s * std::f32::consts::SQRT_2 * g);
                for f in buf.chunks_exact_mut(2) {
                    f[0] *= gi.max(0.0);
                    f[1] *= gd.max(0.0);
                }
            }
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.p.clear();
        self.p.extend_from_slice(p);
    }
    fn limpiar(&mut self) {}
}

impl Procesador for sonido::Moldeador {
    fn procesar(&mut self, buf: &mut [f32]) {
        sonido::Moldeador::procesar(self, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.p = std::array::from_fn(|k| p[k] / 100.0);
    }
    fn limpiar(&mut self) {
        sonido::Moldeador::limpiar(self);
    }
}

/// La voz modula un acorde de sierras que el efecto mismo toca.
struct Voc {
    v: sonido::Vocoder,
    p: Vec<f32>,
    fases: [f64; 8],
    otro: Vec<f32>,
}

impl Voc {
    fn nuevo() -> Voc {
        Voc { v: sonido::Vocoder::nuevo(), p: vec![], fases: std::array::from_fn(|i| i as f64 * 0.13), otro: vec![] }
    }
}

impl Procesador for Voc {
    fn procesar(&mut self, buf: &mut [f32]) {
        self.otro.clear();
        self.otro.resize(buf.len(), 0.0);
        let notas: &[f64] = match self.p[1].round() as i32 {
            0 => &[0.0],
            1 => &[0.0, 7.0],
            2 => &[0.0, 4.0, 7.0],
            3 => &[0.0, 3.0, 7.0],
            _ => &[0.0, 4.0, 7.0, 10.0],
        };
        for (k, st) in notas.iter().enumerate() {
            let hz = 440.0 * 2f64.powf((self.p[0].round() as f64 + st - 69.0) / 12.0);
            // Dos sierras por nota, apenas desafinadas, una a cada lado.
            for (lado, des) in [(0usize, 0.9985), (1, 1.0015)] {
                let paso = hz * des / SR as f64;
                let f = &mut self.fases[k * 2 + lado];
                for x in self.otro.iter_mut().skip(lado).step_by(2) {
                    *f = (*f + paso).fract();
                    *x += ((*f * 2.0 - 1.0) * 0.35) as f32;
                }
            }
        }
        self.v.procesar(buf, &mut self.otro);
        let m = self.p[6] / 100.0;
        buf.iter_mut().zip(&self.otro).for_each(|(x, y)| *x += (y - *x) * m);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.v.ajustar(p[2].round() as usize, p[3], p[4], p[5] / 100.0);
        self.p.clear();
        self.p.extend_from_slice(p);
    }
    fn limpiar(&mut self) {
        self.v.limpiar();
    }
}
