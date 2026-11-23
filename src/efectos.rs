//! Los efectos incluidos. El sonido viene del crate `audio-fx` (GPL-3.0, ver AVISOS.md); aquí se les dan
//! nombres, controles en unidades normales y rangos propios.
use crate::modelo::{SR, db};
use crate::propios;
use audio_fx::effects::{
    Effect, balance, chorus, delay, dynamics as dy, eq, fast_dist, filter, flanger, maximus, phaser, reverb, soft_clipper, stereo_enhancer,
    stereo_shaper, transient_processor as trans, vocoder,
};
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
    /// De Beethoven DAW.
    Parametrico,
    Distorsion,
    Moldeador,
    Vocoder,
    Balance,
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
            TipoFx::Parametrico | TipoFx::Distorsion | TipoFx::Moldeador | TipoFx::Vocoder | TipoFx::Balance => "Beethoven DAW",
        }
    }
    pub const GRUPOS: [&'static str; 7] = ["Beethoven DAW", "Voz", "Dinámica", "Tono y filtros", "Espacio", "Modulación", "Distorsión"];

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
        TipoFx::Ecualizador => Box::new(Eq {
            e: eq::equalizer::Equalizer::new(SR as f64, &ajuste_eq(p).params(), Default::default()),
            l: vec![],
            r: vec![],
        }),
        TipoFx::Compresor | TipoFx::Compuerta | TipoFx::Limitador => {
            let exacto = if tipo == TipoFx::Limitador { dy::FLAG_EXACT_CEILING } else { 0 };
            Box::new(Din {
                tipo,
                d: dy::Dynamics::new(SR as f64, ajuste_din(tipo, p), exacto),
                post: 1.0,
                l: vec![],
                r: vec![],
            })
        }
        TipoFx::Reverberacion => Box::new(Rev(reverb::Reverb::new(SR, Default::default(), 120.0))),
        TipoFx::Eco => Box::new(Eco(delay::Delay::with_settings(SR, &ajuste_eco(p), 120.0))),
        TipoFx::Coro => {
            Box::new(Coro { c: chorus::Chorus::with_settings(SR, &ajuste_coro(p)), mezcla: 0.5, seco: vec![] })
        }
        TipoFx::Faser => Box::new(Fas(phaser::Phaser::with_settings(SR, &ajuste_faser(p)))),
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
        TipoFx::Multibanda => Box::new(Multi(maximus::Maximus::with_state(ajuste_multi(p), SR))),
        TipoFx::Flanger => Box::new(Fla(flanger::Flanger::with_settings(SR, &ajuste_flanger(p)))),
        TipoFx::Filtro => Box::new(Fil(filter::Filter::with_settings(SR, &ajuste_filtro(p)))),
        TipoFx::RecorteSuave => Box::new(Rec { c: soft_clipper::SoftClipper::new(), post: 1.0 }),
        TipoFx::Transitorios => Box::new(Tra { t: trans::TransientProcessor::with_settings(SR, &ajuste_trans(p)), post: 1.0 }),
        TipoFx::Amplitud => Box::new(Amp { a: stereo_enhancer::StereoEnhancer::with_settings(SR, &ajuste_amp(p)), post: 1.0 }),
        TipoFx::Parametrico => Box::new(Par {
            e: eq::equalizer::Equalizer::new(SR as f64, &ajuste_par(p).params(), Default::default()),
            l: vec![],
            r: vec![],
        }),
        TipoFx::Distorsion => Box::new(Dis { f: fast_dist::FastDist::new(), l: vec![], r: vec![] }),
        TipoFx::Moldeador => Box::new(Mol(stereo_shaper::StereoShaper::new(SR))),
        TipoFx::Vocoder => Box::new(Voc::nuevo(p)),
        TipoFx::Balance => Box::new(Bal(balance::Balance::with_settings(SR, &ajuste_bal(p)))),
        TipoFx::Saturador => Box::new(Sat { f: fast_dist::FastDist::new(), mezcla: 0.5, post: 1.0, seco: vec![] }),
    };
    e.ajustar(p);
    e.limpiar();
    e
}

fn separar(buf: &[f32], l: &mut Vec<f32>, r: &mut Vec<f32>) {
    l.clear();
    r.clear();
    for f in buf.chunks_exact(2) {
        l.push(f[0]);
        r.push(f[1]);
    }
}

fn juntar(buf: &mut [f32], l: &[f32], r: &[f32], g: f32) {
    for (i, f) in buf.chunks_exact_mut(2).enumerate() {
        f[0] = l[i] * g;
        f[1] = r[i] * g;
    }
}

// ---- Ecualizador

pub fn ajuste_eq(p: &[f32]) -> eq::Settings {
    let mut s = eq::Settings::default();
    let banda = |db: f32, hz: f32, kind: i32| eq::Band {
        db: db as f64,
        freq: eq::freq_pos(hz as f64),
        width: eq::default_width(kind),
        kind,
        order: 0,
    };
    let nada = banda(0.0, 1000.0, eq::DISABLED);
    s.bands = [
        if p[0] > 20.5 { banda(0.0, p[0], eq::HIGH_PASS) } else { nada },
        banda(p[1], 120.0, eq::LOW_SHELF),
        banda(p[2], p[3], eq::PEAKING),
        banda(p[4], 4000.0, eq::PEAKING),
        banda(p[5], 9000.0, eq::HIGH_SHELF),
        if p[6] < 19900.0 { banda(0.0, p[6], eq::LOW_PASS) } else { nada },
        nada,
    ];
    s.main_db = p[7] as f64;
    s
}

/// Curva del ecualizador en dB, para dibujarla.
pub fn curva_eq(p: &[f32], hz: impl Iterator<Item = f64>) -> Vec<f64> {
    let c = ajuste_eq(p).curve(eq::design::Design::Ordinary(SR as f64));
    hz.map(|h| c.db(h)).collect()
}

struct Eq {
    e: eq::equalizer::Equalizer,
    l: Vec<f32>,
    r: Vec<f32>,
}

impl Procesador for Eq {
    fn procesar(&mut self, buf: &mut [f32]) {
        separar(buf, &mut self.l, &mut self.r);
        self.e.process(&mut self.l, &mut self.r);
        juntar(buf, &self.l, &self.r, 1.0);
    }
    fn ajustar(&mut self, p: &[f32]) {
        let s = ajuste_eq(p);
        let nuevo = s.params();
        for i in 0..nuevo.len() {
            if self.e.params()[i] != nuevo[i] {
                self.e.set_param(i, nuevo[i]);
            }
        }
    }
    fn limpiar(&mut self) {
        self.e.reset();
    }
    fn latencia(&self) -> usize {
        self.e.latency()
    }
}

// ---- Dinámica

/// Valor guardado de una perilla de nivel (inverso de `dynamics::level`).
fn perilla(amp: f32, base: f32, max: f32) -> i32 {
    (1000.0 * (1.0 + (base - 1.0) * amp).ln() / base.ln()).clamp(0.0, max).round() as i32
}

/// Inverso de `dynamics::time_ms`.
fn tiempo(ms: f32) -> i32 {
    (1e4 * (1.0 + ms / 2.0).ln() / 501f32.ln()).clamp(0.0, 1e4).round() as i32
}

fn ajuste_din(tipo: TipoFx, p: &[f32]) -> [i32; dy::PARAMS] {
    let mut v = dy::DEFAULTS;
    match tipo {
        TipoFx::Compresor => {
            v[dy::CEILING] = 2000;
            v[dy::LIM_ATT] = 0;
            v[dy::COMP_THRES] = perilla(db(p[0]), 7.0, 999.0);
            v[dy::COMP_RATIO] = (1000.0 * (1.0 + (p[1] - 1.0) / 0.38).ln() / 51f32.ln()).round() as i32;
            v[dy::COMP_ATT] = tiempo(p[2]);
            v[dy::COMP_REL] = tiempo(p[3]);
            v[dy::RMS_WINDOW] = tiempo(5.0);
        }
        TipoFx::Compuerta => {
            v[dy::CEILING] = 2000;
            v[dy::LIM_ATT] = 0;
            v[dy::GATE_THRES] = perilla(db(p[0]), 7.0, 1000.0);
            v[dy::GATE_GAIN] = perilla(db(p[1]), 7.0, 1000.0);
            v[dy::GATE_REL] = tiempo(p[2]);
        }
        _ => {
            v[dy::GAIN] = perilla(db(p[0]), 7.0, 2000.0);
            v[dy::CEILING] = perilla(db(p[1]), 3.0, 1000.0);
            v[dy::LIM_REL] = tiempo(p[2]);
        }
    }
    v
}

struct Din {
    tipo: TipoFx,
    d: dy::Dynamics,
    post: f32,
    l: Vec<f32>,
    r: Vec<f32>,
}

impl Procesador for Din {
    fn procesar(&mut self, buf: &mut [f32]) {
        separar(buf, &mut self.l, &mut self.r);
        self.d.process(&mut self.l, &mut self.r, None);
        juntar(buf, &self.l, &self.r, self.post);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.post = if self.tipo == TipoFx::Compresor { db(p[4]) } else { 1.0 };
        self.d.set_params(&ajuste_din(self.tipo, p));
    }
    fn limpiar(&mut self) {
        self.d.reset();
    }
    fn latencia(&self) -> usize {
        self.d.latency()
    }
}

// ---- Reverberación

struct Rev(reverb::Reverb);

impl Procesador for Rev {
    fn procesar(&mut self, buf: &mut [f32]) {
        Effect::process(&mut self.0, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        let mut s = reverb::Settings::default();
        let k = &mut s.knobs;
        let mezcla = (p[0] * 1.6).round() as i32;
        k[reverb::WET] = mezcla;
        k[reverb::EARLY] = mezcla;
        k[reverb::DECAY] = (p[1] * 10.0).round() as i32;
        k[reverb::ROOM] = p[2].round() as i32;
        k[reverb::PREDELAY] = (p[3] * 0.384).round() as i32;
        k[reverb::LOW_CUT] = if p[4] <= 20.5 { reverb::LOW_CUT_OFF } else { p[4].round() as i32 };
        k[reverb::HIGH_CUT] = ((p[5] / 100.0).round() as i32).clamp(5, 220);
        k[reverb::DAMPING] = k[reverb::HIGH_CUT];
        self.0.set(s);
    }
    fn limpiar(&mut self) {
        self.0.clear();
    }
}

// ---- Eco

fn ajuste_eco(p: &[f32]) -> delay::Settings {
    let mut s = delay::Settings::default();
    let v = &mut s.v;
    let u = (((p[0] as f64 / 1000.0 - 0.001) / 0.999 / 1.000_216_046_666_079_9) + 0.000216).cbrt();
    v[delay::TIME] = (1.0 + 767.0 * ((u - 0.06) / 0.94).clamp(0.0, 1.0)).round() as i32;
    v[delay::FEEDBACK] = (p[1] * 60.0) as i32;
    v[delay::WET] = (p[2] * 60.0) as i32;
    v[delay::FILTER] = delay::LOW_PASS;
    // Posición del corte en la tabla por tramos del filtro.
    let t = delay::LP;
    let hz = p[3].clamp(t[0], t[4]);
    let i = (0..4).find(|&i| hz <= t[i + 1]).unwrap_or(3);
    let pos = (i as f32 + (hz - t[i]) / (t[i + 1] - t[i])) / 4.0;
    v[delay::CUTOFF] = (pos * 6000.0) as i32;
    v[delay::RESONANCE] = 0;
    v[delay::TYPE] = if p[4] >= 0.5 { delay::PING_PONG } else { delay::TRUE_STEREO };
    s
}

struct Eco(delay::Delay);

impl Procesador for Eco {
    fn procesar(&mut self, buf: &mut [f32]) {
        Effect::process(&mut self.0, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.0.set(&ajuste_eco(p));
    }
    fn limpiar(&mut self) {
        self.0.reset();
    }
}

// ---- Coro, fáser, saturador

fn ajuste_coro(p: &[f32]) -> chorus::Settings {
    let mut s = chorus::Settings::default();
    let v = &mut s.v;
    let vel = |hz: f32| ((hz / 5.0).sqrt() * 5000.0).clamp(0.0, 5000.0) as i32;
    for (i, k) in [1.0, 0.73, 1.31].into_iter().enumerate() {
        v[chorus::SPEED[i]] = vel(p[0] * k);
    }
    v[chorus::DEPTH] = (p[1] * 50.0) as i32;
    v[chorus::DELAY] = ((p[2] - 0.05) / 29.95 * 1024.0) as i32;
    v[chorus::STEREO] = (p[3] * 10.24) as i32;
    v[chorus::CROSS_CUTOFF] = 0;
    v[chorus::WET_ONLY] = 1024;
    s
}

fn mezclar(buf: &mut [f32], seco: &[f32], g_mojado: f32, g_seco: f32) {
    for (b, s) in buf.iter_mut().zip(seco) {
        *b = *b * g_mojado + s * g_seco;
    }
}

struct Coro {
    c: chorus::Chorus,
    mezcla: f32,
    seco: Vec<f32>,
}

impl Procesador for Coro {
    fn procesar(&mut self, buf: &mut [f32]) {
        self.seco.clear();
        self.seco.extend_from_slice(buf);
        Effect::process(&mut self.c, buf);
        mezclar(buf, &self.seco, self.mezcla, 1.0 - self.mezcla * 0.5);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.mezcla = p[4] / 100.0;
        self.c.set(&ajuste_coro(p));
    }
    fn limpiar(&mut self) {
        self.c.start();
    }
}

fn ajuste_faser(p: &[f32]) -> phaser::Settings {
    let mut s = phaser::Settings::default();
    let v = &mut s.v;
    let x = (1.0 + p[0] / 2.0 * (4f32.exp() - 1.0)).ln() / 4.0;
    v[phaser::SWEEP] = (x * 5000.0) as i32;
    v[phaser::MIN_DEPTH] = 100;
    v[phaser::MAX_DEPTH] = 100 + (p[1] * 8.5) as i32;
    v[phaser::FEEDBACK] = (p[2] * 10.0) as i32;
    v[phaser::DRY_WET] = 1024 - (p[3] * 10.24) as i32;
    v[phaser::GAIN] = 2500;
    s
}

struct Fas(phaser::Phaser);

impl Procesador for Fas {
    fn procesar(&mut self, buf: &mut [f32]) {
        Effect::process(&mut self.0, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.0.set(&ajuste_faser(p));
    }
    fn limpiar(&mut self) {
        self.0.start(phaser::IDLE);
    }
}

struct Sat {
    f: fast_dist::FastDist,
    mezcla: f32,
    post: f32,
    seco: Vec<f32>,
}

impl Procesador for Sat {
    fn procesar(&mut self, buf: &mut [f32]) {
        self.seco.clear();
        self.seco.extend_from_slice(buf);
        Effect::process(&mut self.f, buf);
        mezclar(buf, &self.seco, self.mezcla * self.post, (1.0 - self.mezcla) * self.post);
    }
    fn ajustar(&mut self, p: &[f32]) {
        let mut v = fast_dist::DEFAULTS;
        v[fast_dist::PRE] = 128 + (p[0] * 0.64) as i32;
        v[fast_dist::THRESHOLD] = (10.0 - p[0] * 0.05).round() as i32;
        self.f.set(&fast_dist::Settings { v });
        self.mezcla = p[1] / 100.0;
        self.post = db(p[2]);
    }
    fn limpiar(&mut self) {}
}


// ---- Los demás efectos de `audio-fx`

fn ajuste_multi(p: &[f32]) -> maximus::state::State {
    use maximus::state as st;
    let mut s = st::State::new_default();
    let cruce = |hz: f32| (65536.0 * (hz / 30.0).ln() / (16000f32 / 30.0).ln()).round() as i32;
    s.params[st::P_LOW_FREQ] = cruce(p[0]);
    s.params[st::P_HIGH_FREQ] = cruce(p[1]);
    for b in 0..4 {
        s.params[st::band(b, st::B_PRE)] = perilla(db(p[2 + b]), 7.0, 2000.0);
        s.params[st::band(b, st::B_REL)] = tiempo(p[6]);
    }
    s.params[st::band(st::MASTER, st::B_POST)] = perilla(db(p[7]), 7.0, 2000.0);
    s
}

struct Multi(maximus::Maximus);

impl Procesador for Multi {
    fn procesar(&mut self, buf: &mut [f32]) {
        Effect::process(&mut self.0, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        let n = ajuste_multi(p);
        for k in 0..n.params.len() {
            if self.0.state().params[k] != n.params[k] {
                self.0.set_param(k, n.params[k]);
            }
        }
    }
    fn limpiar(&mut self) {
        self.0.reset();
    }
    fn latencia(&self) -> usize {
        self.0.delay()
    }
}

/// Busca la posición (0..max) cuya ley da `v`; las leyes crecen.
fn inversa(max: i32, v: f32, ley: impl Fn(f32) -> f32) -> i32 {
    let (mut a, mut b) = (0, max);
    while a < b {
        let m = (a + b) / 2;
        if ley(m as f32 / max as f32) < v { a = m + 1 } else { b = m }
    }
    a
}

fn ajuste_flanger(p: &[f32]) -> flanger::Settings {
    let mut v = flanger::DEFAULTS;
    let r = |k: usize| flanger::RANGES[k].1;
    v[flanger::RATE] = inversa(r(flanger::RATE), p[0], flanger::speed);
    v[flanger::DEPTH] = inversa(r(flanger::DEPTH), p[1], flanger::ms);
    v[flanger::DELAY] = inversa(r(flanger::DELAY), p[2], flanger::ms);
    v[flanger::FEED] = (p[3] / 100.0 * r(flanger::FEED) as f32) as i32;
    v[flanger::INVERT_FEED] = 0;
    v[flanger::INVERT_WET] = 0;
    v[flanger::DRY] = ((1.0 - p[4] / 200.0) * r(flanger::DRY) as f32 * 0.7) as i32;
    v[flanger::WET] = (p[4] / 100.0 * r(flanger::WET) as f32 * 0.7) as i32;
    v[flanger::CROSS] = 0;
    flanger::Settings { v }
}

struct Fla(flanger::Flanger);

impl Procesador for Fla {
    fn procesar(&mut self, buf: &mut [f32]) {
        Effect::process(&mut self.0, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.0.set(&ajuste_flanger(p));
    }
    fn limpiar(&mut self) {
        self.0.start();
    }
}

fn ajuste_filtro(p: &[f32]) -> filter::Settings {
    let n = |x: f32| (x * 10.24) as i32;
    let corte = inversa(1024, p[0], |t| filter::cutoff_hz((t * 1024.0) as i32, SR as f64) as f32);
    filter::Settings { v: [corte, n(p[1]), n(p[2]), n(p[3]), n(p[4]), 0], center: false }
}

struct Fil(filter::Filter);

impl Procesador for Fil {
    fn procesar(&mut self, buf: &mut [f32]) {
        Effect::process(&mut self.0, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.0.set(&ajuste_filtro(p));
    }
    fn limpiar(&mut self) {
        Effect::reset(&mut self.0);
    }
}

struct Rec {
    c: soft_clipper::SoftClipper,
    post: f32,
}

impl Procesador for Rec {
    fn procesar(&mut self, buf: &mut [f32]) {
        Effect::process(&mut self.c, buf);
        buf.iter_mut().for_each(|x| *x *= self.post);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.c.set(&soft_clipper::Settings { v: [(p[0] * 1.27).round().max(1.0) as i32, 128] });
        self.post = db(p[1]);
    }
    fn limpiar(&mut self) {}
}

fn ajuste_trans(p: &[f32]) -> trans::Settings {
    let mut s = trans::Settings::default();
    // El centro del control es neutro.
    let mitad = |k: usize, x: f32| {
        let (a, b) = trans::RANGES[k];
        a + ((b - a) as f32 * (x + 100.0) / 200.0).round() as i32
    };
    s.v[trans::ATTACK] = mitad(trans::ATTACK, p[0]);
    s.v[trans::RELEASE] = mitad(trans::RELEASE, p[1]);
    s
}

struct Tra {
    t: trans::TransientProcessor,
    post: f32,
}

impl Procesador for Tra {
    fn procesar(&mut self, buf: &mut [f32]) {
        Effect::process(&mut self.t, buf);
        buf.iter_mut().for_each(|x| *x *= self.post);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.t.set(&ajuste_trans(p));
        self.post = db(p[2]);
    }
    fn limpiar(&mut self) {
        Effect::reset(&mut self.t);
    }
}

fn ajuste_amp(p: &[f32]) -> stereo_enhancer::Settings {
    let mut v = stereo_enhancer::DEFAULTS;
    // En `audio-fx`, separar es negativo y juntar es positivo.
    v[stereo_enhancer::SEPARATION] = (-p[0] * 0.96).round() as i32;
    stereo_enhancer::Settings { v }
}

struct Amp {
    a: stereo_enhancer::StereoEnhancer,
    post: f32,
}

impl Procesador for Amp {
    fn procesar(&mut self, buf: &mut [f32]) {
        Effect::process(&mut self.a, buf);
        buf.iter_mut().for_each(|x| *x *= self.post);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.a.set(&ajuste_amp(p));
        self.post = db(p[1]);
    }
    fn limpiar(&mut self) {
        Effect::reset(&mut self.a);
    }
}

// ---- De Beethoven DAW

/// Nombres de los tipos de banda, en el orden del motor.
pub const TIPOS_BANDA: [&str; 8] = ["Apagada", "Pasa graves", "Pasa banda", "Pasa agudos", "Quita banda", "Meseta grave", "Campana", "Meseta aguda"];
pub const ACORDES: [&str; 5] = ["Una nota", "Quinta", "Mayor", "Menor", "Séptima"];

pub fn ajuste_par(p: &[f32]) -> eq::Settings {
    let mut s = eq::Settings::default();
    for (k, b) in s.bands.iter_mut().enumerate() {
        let q = &p[k * 4..k * 4 + 4];
        let kind = (q[0].round() as i32).clamp(0, 7);
        *b = eq::Band { db: if eq::has_level(kind) { q[2] as f64 } else { 0.0 }, freq: eq::freq_pos(q[1] as f64), width: (q[3] as f64 / 100.0).clamp(0.01, 1.0), kind, order: 0 };
    }
    s.main_db = p[28] as f64;
    s
}

/// Curva del paramétrico en dB.
pub fn curva_par(p: &[f32], hz: impl Iterator<Item = f64>) -> Vec<f64> {
    let c = ajuste_par(p).curve(eq::design::Design::Ordinary(SR as f64));
    hz.map(|h| c.db(h)).collect()
}

struct Par {
    e: eq::equalizer::Equalizer,
    l: Vec<f32>,
    r: Vec<f32>,
}

impl Procesador for Par {
    fn procesar(&mut self, buf: &mut [f32]) {
        separar(buf, &mut self.l, &mut self.r);
        self.e.process(&mut self.l, &mut self.r);
        juntar(buf, &self.l, &self.r, 1.0);
    }
    fn ajustar(&mut self, p: &[f32]) {
        let s = ajuste_par(p);
        let nuevo = s.params();
        for i in 0..nuevo.len() {
            if self.e.params()[i] != nuevo[i] {
                self.e.set_param(i, nuevo[i]);
            }
        }
    }
    fn limpiar(&mut self) {
        self.e.reset();
    }
    fn latencia(&self) -> usize {
        self.e.latency()
    }
}

struct Dis {
    f: fast_dist::FastDist,
    l: Vec<f32>,
    r: Vec<f32>,
}

impl Procesador for Dis {
    fn procesar(&mut self, buf: &mut [f32]) {
        separar(buf, &mut self.l, &mut self.r);
        self.f.process(&mut self.l, &mut self.r);
        juntar(buf, &self.l, &self.r, 1.0);
    }
    fn ajustar(&mut self, p: &[f32]) {
        let mut v = fast_dist::DEFAULTS;
        v[fast_dist::PRE] = (128.0 + p[0] * 0.64) as i32;
        v[fast_dist::THRESHOLD] = p[1].round() as i32;
        v[fast_dist::TYPE] = (p[2] >= 0.5) as i32;
        v[fast_dist::MIX] = (p[3] * 1.28) as i32;
        v[fast_dist::POST] = (p[4] * 1.28) as i32;
        self.f.set(&fast_dist::Settings { v });
    }
    fn limpiar(&mut self) {}
}

struct Mol(stereo_shaper::StereoShaper);

impl Procesador for Mol {
    fn procesar(&mut self, buf: &mut [f32]) {
        Effect::process(&mut self.0, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        use stereo_shaper as m;
        let mut v = m::DEFAULTS;
        for (k, i) in [m::RIGHT_INTO_LEFT, m::LEFT, m::RIGHT, m::LEFT_INTO_RIGHT, m::DELAY, m::PHASE].into_iter().enumerate() {
            let (a, z) = m::RANGES[i];
            // 100 % es el nivel de siempre en los cuatro caminos, y el extremo en retraso y fase.
            let tope = if k < 4 { m::UNITY } else if p[k] < 0.0 { -a } else { z };
            v[i] = ((p[k] / 100.0 * tope as f32) as i32).clamp(a, z);
        }
        self.0.set(&m::Settings { v });
    }
    fn limpiar(&mut self) {
        self.0.flush();
    }
}

fn ajuste_bal(p: &[f32]) -> balance::Settings {
    let mut v = balance::DEFAULTS;
    v[balance::BALANCE] = (p[0] * 1.28) as i32;
    v[balance::VOLUME] = (p[1] * 2.56) as i32;
    balance::Settings { v }
}

struct Bal(balance::Balance);

impl Procesador for Bal {
    fn procesar(&mut self, buf: &mut [f32]) {
        Effect::process(&mut self.0, buf);
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.0.set(&ajuste_bal(p));
    }
    fn limpiar(&mut self) {
        self.0.flush();
    }
}

/// La voz modula un acorde de sierras que el efecto mismo toca.
struct Voc {
    v: vocoder::Vocoder,
    p: Vec<f32>,
    fases: [f64; 8],
    l: Vec<f32>,
    r: Vec<f32>,
    pl: Vec<f32>,
    pr: Vec<f32>,
    sl: Vec<f32>,
    sr: Vec<f32>,
}

impl Voc {
    fn nuevo(p: &[f32]) -> Voc {
        Voc { v: vocoder::Vocoder::new(SR, Self::params(p)), p: p.to_vec(), fases: std::array::from_fn(|i| i as f64 * 0.13), l: vec![], r: vec![], pl: vec![], pr: vec![], sl: vec![], sr: vec![] }
    }

    fn params(p: &[f32]) -> vocoder::Params {
        vocoder::Params {
            bands: p[2].round() as usize,
            attack_ms: p[3] as f64,
            release_ms: p[4] as f64,
            noise: p[5] as f64 / 100.0 * 0.6,
            // Las consonantes pasan solas por arriba.
            mod_level: p[5] as f64 / 100.0 * 0.5,
            ..Default::default()
        }
    }
}

impl Procesador for Voc {
    fn procesar(&mut self, buf: &mut [f32]) {
        separar(buf, &mut self.l, &mut self.r);
        let n = self.l.len();
        for v in [&mut self.pl, &mut self.pr, &mut self.sl, &mut self.sr] {
            v.clear();
            v.resize(n, 0.0);
        }
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
                let sal = if lado == 0 { &mut self.pl } else { &mut self.pr };
                for x in sal.iter_mut() {
                    *f = (*f + paso).fract();
                    *x += ((*f * 2.0 - 1.0) * 0.35) as f32;
                }
            }
        }
        self.v.process((&self.l, &self.r), (&self.pl, &self.pr), (&mut self.sl, &mut self.sr));
        let m = self.p[6] / 100.0;
        for (i, f) in buf.chunks_exact_mut(2).enumerate() {
            f[0] += (self.sl[i] - f[0]) * m;
            f[1] += (self.sr[i] - f[1]) * m;
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        if Self::params(p) != Self::params(&self.p) {
            self.v.set(Self::params(p));
        }
        self.p = p.to_vec();
    }
    fn limpiar(&mut self) {
        self.v.reset();
    }
}
