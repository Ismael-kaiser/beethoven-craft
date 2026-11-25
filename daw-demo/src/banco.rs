// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Los instrumentos de fábrica: grabaciones nota por nota del banco MuseScore General (.sf3, MIT; ver AVISOS.md).
//! Un canal guarda solo la clave («piano», «bombo»…); el banco se busca junto al programa.
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const ARCHIVO: &str = "MuseScore_General.sf3";

pub struct Instrumento {
    pub clave: &'static str,
    /// En español; se traduce con `idioma::t`.
    pub familia: &'static str,
    pub nombre: &'static str,
    /// Nombre del preset en el banco.
    pub preset: &'static str,
    /// Las piezas de batería suenan siempre en su tecla del juego: el bombo es la 36.
    pub nota: Option<u8>,
}

use Instrumento as I;

pub const INSTRUMENTOS: &[Instrumento] = &[
    I { clave: "piano", familia: "Pianos", nombre: "Piano de cola", preset: "Grand Piano", nota: None },
    I { clave: "piano_suave", familia: "Pianos", nombre: "Piano de cola suave", preset: "Mellow Grand Piano", nota: None },
    I { clave: "piano_brillante", familia: "Pianos", nombre: "Piano brillante", preset: "Bright Grand Piano", nota: None },
    I { clave: "piano_electrico_cola", familia: "Pianos", nombre: "Piano eléctrico de cola", preset: "Electric Grand", nota: None },
    I { clave: "piano_cantina", familia: "Pianos", nombre: "Piano de cantina", preset: "Honky-Tonk Piano", nota: None },
    I { clave: "piano_electrico", familia: "Pianos", nombre: "Piano eléctrico", preset: "Tine Electric Piano", nota: None },
    I { clave: "piano_fm", familia: "Pianos", nombre: "Piano eléctrico FM", preset: "FM Electric Piano", nota: None },
    I { clave: "clavecin", familia: "Pianos", nombre: "Clavecín", preset: "Harpsichord", nota: None },
    I { clave: "clavinet", familia: "Pianos", nombre: "Clavinet", preset: "Clavinet", nota: None },
    I { clave: "celesta", familia: "Percusión afinada", nombre: "Celesta", preset: "Celesta", nota: None },
    I { clave: "glockenspiel", familia: "Percusión afinada", nombre: "Glockenspiel", preset: "Glockenspiel", nota: None },
    I { clave: "caja_musical", familia: "Percusión afinada", nombre: "Caja de música", preset: "Music Box", nota: None },
    I { clave: "campana", familia: "Percusión afinada", nombre: "Vibráfono", preset: "Vibraphone", nota: None },
    I { clave: "marimba", familia: "Percusión afinada", nombre: "Marimba", preset: "Marimba", nota: None },
    I { clave: "xilofono", familia: "Percusión afinada", nombre: "Xilófono", preset: "Xylophone", nota: None },
    I { clave: "campanas_tubulares", familia: "Percusión afinada", nombre: "Campanas tubulares", preset: "Tubular Bells", nota: None },
    I { clave: "campana_iglesia", familia: "Percusión afinada", nombre: "Campana de iglesia", preset: "Church Bell", nota: None },
    I { clave: "salterio", familia: "Percusión afinada", nombre: "Salterio", preset: "Dulcimer", nota: None },
    I { clave: "timbales", familia: "Percusión afinada", nombre: "Timbales de orquesta", preset: "Timpani", nota: None },
    I { clave: "organo", familia: "Órganos y acordeones", nombre: "Órgano", preset: "Drawbar Organ", nota: None },
    I { clave: "organo_percusivo", familia: "Órganos y acordeones", nombre: "Órgano percusivo", preset: "Percussive Organ", nota: None },
    I { clave: "organo_rock", familia: "Órganos y acordeones", nombre: "Órgano de rock", preset: "Rock Organ", nota: None },
    I { clave: "organo_iglesia", familia: "Órganos y acordeones", nombre: "Órgano de iglesia", preset: "Church Organ", nota: None },
    I { clave: "organo_lengueta", familia: "Órganos y acordeones", nombre: "Armonio", preset: "Reed Organ", nota: None },
    I { clave: "acordeon", familia: "Órganos y acordeones", nombre: "Acordeón", preset: "Accordion", nota: None },
    I { clave: "acordeon_italiano", familia: "Órganos y acordeones", nombre: "Acordeón italiano", preset: "Italian Accordion", nota: None },
    I { clave: "armonica", familia: "Órganos y acordeones", nombre: "Armónica", preset: "Harmonica", nota: None },
    I { clave: "bandoneon", familia: "Órganos y acordeones", nombre: "Bandoneón", preset: "Bandoneon", nota: None },
    I { clave: "cuerda", familia: "Guitarras", nombre: "Guitarra clásica", preset: "Nylon String Guitar", nota: None },
    I { clave: "guitarra_acustica", familia: "Guitarras", nombre: "Guitarra acústica", preset: "Steel String Guitar", nota: None },
    I { clave: "guitarra_12", familia: "Guitarras", nombre: "Guitarra de 12 cuerdas", preset: "12-String Guitar", nota: None },
    I { clave: "guitarra_jazz", familia: "Guitarras", nombre: "Guitarra de jazz", preset: "Jazz Guitar", nota: None },
    I { clave: "guitarra_limpia", familia: "Guitarras", nombre: "Guitarra eléctrica", preset: "Clean Guitar", nota: None },
    I { clave: "guitarra_funk", familia: "Guitarras", nombre: "Guitarra funk", preset: "Funk Guitar", nota: None },
    I { clave: "guitarra_apagada", familia: "Guitarras", nombre: "Guitarra apagada", preset: "Palm Muted Guitar", nota: None },
    I { clave: "guitarra_saturada", familia: "Guitarras", nombre: "Guitarra saturada", preset: "Overdrive Guitar", nota: None },
    I { clave: "guitarra_distorsion", familia: "Guitarras", nombre: "Guitarra con distorsión", preset: "Distortion Guitar", nota: None },
    I { clave: "guitarra_armonicos", familia: "Guitarras", nombre: "Armónicos de guitarra", preset: "Guitar Harmonics", nota: None },
    I { clave: "guitarra_hawaiana", familia: "Guitarras", nombre: "Guitarra hawaiana", preset: "Hawaiian Guitar", nota: None },
    I { clave: "ukelele", familia: "Guitarras", nombre: "Ukelele", preset: "Ukulele", nota: None },
    I { clave: "mandolina", familia: "Guitarras", nombre: "Mandolina", preset: "Mandolin", nota: None },
    I { clave: "banjo", familia: "Guitarras", nombre: "Banjo", preset: "Banjo", nota: None },
    I { clave: "bajo", familia: "Bajos", nombre: "Bajo eléctrico", preset: "Fingered Bass", nota: None },
    I { clave: "contrabajo_jazz", familia: "Bajos", nombre: "Contrabajo de jazz", preset: "Acoustic Bass", nota: None },
    I { clave: "bajo_pua", familia: "Bajos", nombre: "Bajo con púa", preset: "Picked Bass", nota: None },
    I { clave: "bajo_sin_trastes", familia: "Bajos", nombre: "Bajo sin trastes", preset: "Fretless Bass", nota: None },
    I { clave: "bajo_slap", familia: "Bajos", nombre: "Bajo slap", preset: "Slap Bass", nota: None },
    I { clave: "bajo_pop", familia: "Bajos", nombre: "Bajo pop", preset: "Pop Bass", nota: None },
    I { clave: "bajo_sinte", familia: "Bajos", nombre: "Bajo sintetizado", preset: "Synth Bass 1", nota: None },
    I { clave: "bajo_sinte_2", familia: "Bajos", nombre: "Bajo sintetizado 2", preset: "Synth Bass 2", nota: None },
    I { clave: "bajo_sinte_3", familia: "Bajos", nombre: "Bajo sintetizado 3", preset: "Synth Bass 3", nota: None },
    I { clave: "violin", familia: "Cuerdas", nombre: "Violín", preset: "Violin", nota: None },
    I { clave: "violin_lento", familia: "Cuerdas", nombre: "Violín lento", preset: "Slow Violin", nota: None },
    I { clave: "viola", familia: "Cuerdas", nombre: "Viola", preset: "Viola", nota: None },
    I { clave: "violonchelo", familia: "Cuerdas", nombre: "Violonchelo", preset: "Cello", nota: None },
    I { clave: "contrabajo", familia: "Cuerdas", nombre: "Contrabajo", preset: "Contrabass", nota: None },
    I { clave: "arpa", familia: "Cuerdas", nombre: "Arpa", preset: "Harp", nota: None },
    I { clave: "violin_country", familia: "Cuerdas", nombre: "Violín country", preset: "Fiddle", nota: None },
    I { clave: "orquesta_cuerdas", familia: "Orquesta", nombre: "Orquesta de cuerdas", preset: "Strings Fast", nota: None },
    I { clave: "cuerdas_lentas", familia: "Orquesta", nombre: "Cuerdas lentas", preset: "Strings Slow", nota: None },
    I { clave: "cuerdas_tremolo", familia: "Orquesta", nombre: "Cuerdas en trémolo", preset: "Strings Tremolo", nota: None },
    I { clave: "cuerdas_pizzicato", familia: "Orquesta", nombre: "Cuerdas en pizzicato", preset: "Strings Pizzicato", nota: None },
    I { clave: "violines", familia: "Orquesta", nombre: "Violines", preset: "Violins Fast", nota: None },
    I { clave: "violines_lentos", familia: "Orquesta", nombre: "Violines lentos", preset: "Violins Slow", nota: None },
    I { clave: "violines_pizzicato", familia: "Orquesta", nombre: "Violines en pizzicato", preset: "Violins Pizzicato", nota: None },
    I { clave: "violines_tremolo", familia: "Orquesta", nombre: "Violines en trémolo", preset: "Violins Tremolo", nota: None },
    I { clave: "violas", familia: "Orquesta", nombre: "Violas", preset: "Violas Slow", nota: None },
    I { clave: "violonchelos", familia: "Orquesta", nombre: "Violonchelos", preset: "Celli Slow", nota: None },
    I { clave: "violonchelos_pizzicato", familia: "Orquesta", nombre: "Violonchelos en pizzicato", preset: "Celli Pizzicato", nota: None },
    I { clave: "contrabajos", familia: "Orquesta", nombre: "Contrabajos", preset: "Basses Slow", nota: None },
    I { clave: "contrabajos_pizzicato", familia: "Orquesta", nombre: "Contrabajos en pizzicato", preset: "Basses Pizzicato", nota: None },
    I { clave: "golpe_orquesta", familia: "Orquesta", nombre: "Golpe de orquesta", preset: "Orchestra Hit", nota: None },
    I { clave: "pad_orquesta", familia: "Orquesta", nombre: "Fondo de orquesta", preset: "Orchestral Pad", nota: None },
    I { clave: "coro", familia: "Coros y voces", nombre: "Coro de voces", preset: "Choir Aahs", nota: None },
    I { clave: "voces_oo", familia: "Coros y voces", nombre: "Voces «u»", preset: "Voice Oohs", nota: None },
    I { clave: "voz_sinte", familia: "Coros y voces", nombre: "Voz sintetizada", preset: "Synth Voice", nota: None },
    I { clave: "voz_solista", familia: "Coros y voces", nombre: "Voz solista", preset: "Solo Vox", nota: None },
    I { clave: "trompeta", familia: "Metales", nombre: "Trompeta", preset: "Trumpet", nota: None },
    I { clave: "trompeta_sordina", familia: "Metales", nombre: "Trompeta con sordina", preset: "Harmon Mute Trumpet", nota: None },
    I { clave: "trombon", familia: "Metales", nombre: "Trombón", preset: "Trombone", nota: None },
    I { clave: "tuba", familia: "Metales", nombre: "Tuba", preset: "Tuba", nota: None },
    I { clave: "cornos", familia: "Metales", nombre: "Cornos franceses", preset: "French Horns", nota: None },
    I { clave: "metales", familia: "Metales", nombre: "Sección de metales", preset: "Brass Section", nota: None },
    I { clave: "metales_2", familia: "Metales", nombre: "Sección de metales 2", preset: "Brass 2", nota: None },
    I { clave: "metales_sinte", familia: "Metales", nombre: "Metales sintetizados", preset: "Synth Brass 1", nota: None },
    I { clave: "metales_sinte_2", familia: "Metales", nombre: "Metales sintetizados 2", preset: "Synth Brass 2", nota: None },
    I { clave: "sax_soprano", familia: "Maderas", nombre: "Saxofón soprano", preset: "Soprano Sax", nota: None },
    I { clave: "sax_alto", familia: "Maderas", nombre: "Saxofón alto", preset: "Alto Sax", nota: None },
    I { clave: "sax_tenor", familia: "Maderas", nombre: "Saxofón tenor", preset: "Tenor Sax", nota: None },
    I { clave: "sax_baritono", familia: "Maderas", nombre: "Saxofón barítono", preset: "Baritone Sax", nota: None },
    I { clave: "oboe", familia: "Maderas", nombre: "Oboe", preset: "Oboe", nota: None },
    I { clave: "corno_ingles", familia: "Maderas", nombre: "Corno inglés", preset: "English Horn", nota: None },
    I { clave: "fagot", familia: "Maderas", nombre: "Fagot", preset: "Bassoon", nota: None },
    I { clave: "clarinete", familia: "Maderas", nombre: "Clarinete", preset: "Clarinet", nota: None },
    I { clave: "flautin", familia: "Maderas", nombre: "Flautín", preset: "Piccolo", nota: None },
    I { clave: "flauta", familia: "Maderas", nombre: "Flauta", preset: "Flute", nota: None },
    I { clave: "flauta_dulce", familia: "Maderas", nombre: "Flauta dulce", preset: "Recorder", nota: None },
    I { clave: "flauta_pan", familia: "Maderas", nombre: "Flauta de pan", preset: "Pan Flute", nota: None },
    I { clave: "botella", familia: "Maderas", nombre: "Botella soplada", preset: "Bottle Chiff", nota: None },
    I { clave: "shakuhachi", familia: "Maderas", nombre: "Shakuhachi", preset: "Shakuhachi", nota: None },
    I { clave: "silbido", familia: "Maderas", nombre: "Silbido", preset: "Whistle", nota: None },
    I { clave: "ocarina", familia: "Maderas", nombre: "Ocarina", preset: "Ocarina", nota: None },
    I { clave: "clasico", familia: "Sintetizadores", nombre: "Sintetizador cuadrado", preset: "Square Lead", nota: None },
    I { clave: "sinte_sierra", familia: "Sintetizadores", nombre: "Sintetizador sierra", preset: "Saw Lead", nota: None },
    I { clave: "sinte_caliope", familia: "Sintetizadores", nombre: "Calíope", preset: "Calliope Lead", nota: None },
    I { clave: "sinte_chiff", familia: "Sintetizadores", nombre: "Sintetizador soplado", preset: "Chiffer Lead", nota: None },
    I { clave: "sinte_charang", familia: "Sintetizadores", nombre: "Charango eléctrico", preset: "Charang", nota: None },
    I { clave: "sinte_quintas", familia: "Sintetizadores", nombre: "Sierra en quintas", preset: "5th Saw Wave", nota: None },
    I { clave: "sinte_bajo_solista", familia: "Sintetizadores", nombre: "Bajo y solista", preset: "Bass & Lead", nota: None },
    I { clave: "sinte_seno", familia: "Sintetizadores", nombre: "Onda senoidal", preset: "Sine Wave", nota: None },
    I { clave: "sinte_desafinado", familia: "Sintetizadores", nombre: "Sierra desafinada", preset: "Detuned Saw", nota: None },
    I { clave: "colchon", familia: "Fondos", nombre: "Fondo cálido", preset: "Warm Pad", nota: None },
    I { clave: "fondo_fantasia", familia: "Fondos", nombre: "Fantasía", preset: "Fantasia", nota: None },
    I { clave: "fondo_poli", familia: "Fondos", nombre: "Polisintetizador", preset: "Polysynth", nota: None },
    I { clave: "fondo_espacial", familia: "Fondos", nombre: "Voz espacial", preset: "Space Voice", nota: None },
    I { clave: "fondo_cristal", familia: "Fondos", nombre: "Cristal frotado", preset: "Bowed Glass", nota: None },
    I { clave: "fondo_metal", familia: "Fondos", nombre: "Fondo metálico", preset: "Metal Pad", nota: None },
    I { clave: "fondo_halo", familia: "Fondos", nombre: "Halo", preset: "Halo Pad", nota: None },
    I { clave: "fondo_barrido", familia: "Fondos", nombre: "Barrido", preset: "Sweep Pad", nota: None },
    I { clave: "cuerdas_sinte", familia: "Fondos", nombre: "Cuerdas sintetizadas", preset: "Synth Strings 1", nota: None },
    I { clave: "cuerdas_sinte_2", familia: "Fondos", nombre: "Cuerdas sintetizadas 2", preset: "Synth Strings 2", nota: None },
    I { clave: "lluvia_hielo", familia: "Ambientes", nombre: "Lluvia de hielo", preset: "Ice Rain", nota: None },
    I { clave: "banda_sonora", familia: "Ambientes", nombre: "Banda sonora", preset: "Soundtrack", nota: None },
    I { clave: "cristal", familia: "Ambientes", nombre: "Cristal", preset: "Crystal", nota: None },
    I { clave: "atmosfera", familia: "Ambientes", nombre: "Atmósfera", preset: "Atmosphere", nota: None },
    I { clave: "brillo", familia: "Ambientes", nombre: "Brillo", preset: "Brightness", nota: None },
    I { clave: "duendes", familia: "Ambientes", nombre: "Duendes", preset: "Goblin", nota: None },
    I { clave: "gotas_eco", familia: "Ambientes", nombre: "Gotas con eco", preset: "Echo Drops", nota: None },
    I { clave: "estelar", familia: "Ambientes", nombre: "Tema estelar", preset: "Star Theme", nota: None },
    I { clave: "sitar", familia: "Del mundo", nombre: "Sitar", preset: "Sitar", nota: None },
    I { clave: "shamisen", familia: "Del mundo", nombre: "Shamisen", preset: "Shamisen", nota: None },
    I { clave: "koto", familia: "Del mundo", nombre: "Koto", preset: "Koto", nota: None },
    I { clave: "koto_taisho", familia: "Del mundo", nombre: "Koto Taishō", preset: "Taisho Koto", nota: None },
    I { clave: "kalimba", familia: "Del mundo", nombre: "Kalimba", preset: "Kalimba", nota: None },
    I { clave: "gaita", familia: "Del mundo", nombre: "Gaita", preset: "Bagpipe", nota: None },
    I { clave: "shehnai", familia: "Del mundo", nombre: "Shehnai", preset: "Shenai", nota: None },
    I { clave: "tambores_acero", familia: "Del mundo", nombre: "Tambores de acero", preset: "Steel Drums", nota: None },
    I { clave: "taiko", familia: "Del mundo", nombre: "Taiko", preset: "Taiko Drum", nota: None },
    I { clave: "bombo", familia: "Batería", nombre: "Bombo", preset: "Standard", nota: Some(36) },
    I { clave: "caja", familia: "Batería", nombre: "Caja", preset: "Standard", nota: Some(38) },
    I { clave: "aro", familia: "Batería", nombre: "Aro", preset: "Standard", nota: Some(37) },
    I { clave: "aplauso", familia: "Batería", nombre: "Aplauso", preset: "Standard", nota: Some(39) },
    I { clave: "platillo", familia: "Batería", nombre: "Platillo", preset: "Standard", nota: Some(42) },
    I { clave: "platillo_abierto", familia: "Batería", nombre: "Platillo abierto", preset: "Standard", nota: Some(46) },
    I { clave: "platillo_pie", familia: "Batería", nombre: "Platillo de pie", preset: "Standard", nota: Some(44) },
    I { clave: "crash", familia: "Batería", nombre: "Crash", preset: "Standard", nota: Some(49) },
    I { clave: "ride", familia: "Batería", nombre: "Ride", preset: "Standard", nota: Some(51) },
    I { clave: "china", familia: "Batería", nombre: "China", preset: "Standard", nota: Some(52) },
    I { clave: "tom_grave", familia: "Batería", nombre: "Tom grave", preset: "Standard", nota: Some(41) },
    I { clave: "tom_medio", familia: "Batería", nombre: "Tom medio", preset: "Standard", nota: Some(45) },
    I { clave: "tom_agudo", familia: "Batería", nombre: "Tom agudo", preset: "Standard", nota: Some(48) },
    I { clave: "bombo_808", familia: "Batería", nombre: "Bombo 808", preset: "TR-808", nota: Some(36) },
    I { clave: "caja_808", familia: "Batería", nombre: "Caja 808", preset: "TR-808", nota: Some(38) },
    I { clave: "aplauso_808", familia: "Batería", nombre: "Aplauso 808", preset: "TR-808", nota: Some(39) },
    I { clave: "platillo_808", familia: "Batería", nombre: "Platillo 808", preset: "TR-808", nota: Some(42) },
    I { clave: "cencerro_808", familia: "Batería", nombre: "Cencerro 808", preset: "TR-808", nota: Some(56) },
    I { clave: "pandereta", familia: "Percusión", nombre: "Pandereta", preset: "Standard", nota: Some(54) },
    I { clave: "cencerro", familia: "Percusión", nombre: "Cencerro", preset: "Standard", nota: Some(56) },
    I { clave: "bongo_agudo", familia: "Percusión", nombre: "Bongó agudo", preset: "Standard", nota: Some(60) },
    I { clave: "bongo_grave", familia: "Percusión", nombre: "Bongó grave", preset: "Standard", nota: Some(61) },
    I { clave: "conga", familia: "Percusión", nombre: "Conga", preset: "Standard", nota: Some(63) },
    I { clave: "conga_grave", familia: "Percusión", nombre: "Conga grave", preset: "Standard", nota: Some(64) },
    I { clave: "timbal", familia: "Percusión", nombre: "Timbal", preset: "Standard", nota: Some(65) },
    I { clave: "agogo", familia: "Percusión", nombre: "Agogó", preset: "Agogo", nota: None },
    I { clave: "cabasa", familia: "Percusión", nombre: "Cabasa", preset: "Standard", nota: Some(69) },
    I { clave: "maracas", familia: "Percusión", nombre: "Maracas", preset: "Standard", nota: Some(70) },
    I { clave: "guiro", familia: "Percusión", nombre: "Güiro", preset: "Standard", nota: Some(73) },
    I { clave: "claves", familia: "Percusión", nombre: "Claves", preset: "Standard", nota: Some(75) },
    I { clave: "triangulo", familia: "Percusión", nombre: "Triángulo", preset: "Standard", nota: Some(81) },
    I { clave: "castanuelas", familia: "Percusión", nombre: "Castañuelas", preset: "Castanets", nota: None },
    I { clave: "bloque_madera", familia: "Percusión", nombre: "Bloque de madera", preset: "Woodblock", nota: None },
    I { clave: "bloques_templo", familia: "Percusión", nombre: "Bloques de templo", preset: "Temple Blocks", nota: None },
    I { clave: "campanita", familia: "Percusión", nombre: "Campanita", preset: "Tinker Bell", nota: None },
    I { clave: "tom_melodico", familia: "Percusión", nombre: "Tom melódico", preset: "Melodic Tom", nota: None },
    I { clave: "tambor_sinte", familia: "Percusión", nombre: "Tambor sintetizado", preset: "Synth Drum", nota: None },
    I { clave: "bombo_concierto", familia: "Percusión", nombre: "Bombo de concierto", preset: "Concert Bass Drum", nota: None },
    I { clave: "platillo_al_reves", familia: "Percusión", nombre: "Platillo al revés", preset: "Reverse Cymbal", nota: None },
    I { clave: "bateria", familia: "Baterías completas", nombre: "Batería estándar", preset: "Standard", nota: None },
    I { clave: "bateria_sala", familia: "Baterías completas", nombre: "Batería de sala", preset: "Room", nota: None },
    I { clave: "bateria_potente", familia: "Baterías completas", nombre: "Batería potente", preset: "Power", nota: None },
    I { clave: "bateria_electronica", familia: "Baterías completas", nombre: "Batería electrónica", preset: "Electronic", nota: None },
    I { clave: "bateria_808", familia: "Baterías completas", nombre: "Batería 808", preset: "TR-808", nota: None },
    I { clave: "bateria_jazz", familia: "Baterías completas", nombre: "Batería de jazz", preset: "Jazz", nota: None },
    I { clave: "bateria_escobillas", familia: "Baterías completas", nombre: "Batería con escobillas", preset: "Brush", nota: None },
    I { clave: "bateria_orquesta", familia: "Baterías completas", nombre: "Percusión de orquesta", preset: "Orchestra Kit", nota: None },
    I { clave: "banda_marcha", familia: "Baterías completas", nombre: "Caja de banda de marcha", preset: "Marching Snare", nota: None },
    I { clave: "trastes", familia: "Efectos de sonido", nombre: "Ruido de trastes", preset: "Fret Noise", nota: None },
    I { clave: "respiracion", familia: "Efectos de sonido", nombre: "Respiración", preset: "Breath Noise", nota: None },
    I { clave: "mar", familia: "Efectos de sonido", nombre: "Mar", preset: "Sea Shore", nota: None },
    I { clave: "pajaros", familia: "Efectos de sonido", nombre: "Pájaros", preset: "Bird Tweet", nota: None },
    I { clave: "telefono", familia: "Efectos de sonido", nombre: "Teléfono", preset: "Telephone", nota: None },
    I { clave: "helicoptero", familia: "Efectos de sonido", nombre: "Helicóptero", preset: "Helicopter", nota: None },
    I { clave: "aplausos", familia: "Efectos de sonido", nombre: "Aplausos de público", preset: "Applause", nota: None },
    I { clave: "disparo", familia: "Efectos de sonido", nombre: "Disparo", preset: "Gun Shot", nota: None },
    I { clave: "retroalimentacion", familia: "Efectos de sonido", nombre: "Retroalimentación de guitarra", preset: "Guitar Feedback", nota: None },
];

pub fn de(clave: &str) -> Option<&'static Instrumento> {
    INSTRUMENTOS.iter().find(|x| x.clave == clave)
}

/// Las familias en su orden, con sus instrumentos.
pub fn familias() -> Vec<(&'static str, Vec<&'static Instrumento>)> {
    let mut v: Vec<(&str, Vec<&Instrumento>)> = vec![];
    for x in INSTRUMENTOS {
        match v.last_mut() {
            Some((f, l)) if *f == x.familia => l.push(x),
            _ => v.push((x.familia, vec![x])),
        }
    }
    v
}

/// La carpeta `sonidos` junto al programa, en los Resources de la app de Mac o, al compilar, en `daw-demo/sonidos`.
pub fn archivo() -> Option<&'static Path> {
    static R: OnceLock<Option<PathBuf>> = OnceLock::new();
    R.get_or_init(|| {
        let exe = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
        let mut v: Vec<PathBuf> = std::env::var_os("BDD_SONIDOS").map(PathBuf::from).into_iter().collect();
        if let Some(d) = exe {
            v.push(d.join("sonidos"));
            v.push(d.join("../Resources/sonidos"));
        }
        v.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("sonidos"));
        v.into_iter().map(|d| d.join(ARCHIVO)).find(|r| r.is_file())
    })
    .as_deref()
}

/// En qué lugar del banco está ese preset.
pub fn indice(preset: &str) -> Option<usize> {
    static P: OnceLock<Vec<String>> = OnceLock::new();
    P.get_or_init(|| archivo().map(crate::muestrario::presets).unwrap_or_default()).iter().position(|n| n == preset)
}
