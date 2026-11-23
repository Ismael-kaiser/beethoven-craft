//! Estilos listos: varios efectos de video con sus valores y, en algunos, animación.
use crate::composicion::fx_nuevo;
use crate::modelo::*;

/// Un control: nombre y valor al principio y al final del clip (iguales = fijo).
type C = (&'static str, f64, f64);

pub struct Estilo {
    pub nombre: &'static str,
    pub ayuda: &'static str,
    pub fx: &'static [(&'static str, &'static [C])],
    /// Escala al principio y al final, y giro.
    escala: Option<(f64, f64)>,
}

const fn e(nombre: &'static str, ayuda: &'static str, fx: &'static [(&'static str, &'static [C])], escala: Option<(f64, f64)>) -> Estilo {
    Estilo { nombre, ayuda, fx, escala }
}

const LUM: &str = "ec.color.lumetri";

pub const ESTILOS: [Estilo; 32] = [
    e("Cinta VHS", "Colores corridos, ruido y rayas", &[("fc:rgb_split", &[("amount", 9.0, 9.0)]), ("ec.noise.noise", &[("amount", 14.0, 14.0)]), (LUM, &[("basicCorrection/saturation", 125.0, 125.0), ("creative/adjustments/fadedFilm", 35.0, 35.0)]), ("ec.blur.channel", &[("redBlurriness", 10.0, 10.0), ("blueBlurriness", 14.0, 14.0)])], None),
    e("Película vieja", "Sepia, grano y viñeta", &[("ec.color.tint", &[("amount", 70.0, 70.0)]), ("ec.noise.noise", &[("amount", 18.0, 18.0)]), (LUM, &[("creative/adjustments/fadedFilm", 60.0, 60.0), ("basicCorrection/whiteBalance/temperature", 40.0, 40.0), ("vignette/vignetteAmount", -2.5, -2.5)])], None),
    e("Cine", "Contraste, sombras suaves y viñeta", &[(LUM, &[("basicCorrection/tone/contrast", 30.0, 30.0), ("basicCorrection/tone/highlights", -25.0, -25.0), ("basicCorrection/tone/shadows", 15.0, 15.0), ("basicCorrection/saturation", 85.0, 85.0), ("vignette/vignetteAmount", -1.5, -1.5)])], None),
    e("Sueño", "Brillo suave y difuso", &[("fc:wonder_glow", &[]), ("ec.blur.gaussian", &[("blurriness", 4.0, 4.0)]), (LUM, &[("basicCorrection/tone/exposure", 0.4, 0.4)])], None),
    e("Falla digital", "Colores separados y bloques", &[("fc:rgb_split", &[("amount", 22.0, 22.0)]), ("ec.vr.digitalglitch", &[])], None),
    e("Neón", "Bordes que brillan sobre oscuro", &[(LUM, &[("basicCorrection/tone/exposure", -1.2, -1.2), ("basicCorrection/saturation", 140.0, 140.0)]), ("fc:edge_glow", &[])], None),
    e("Blanco y negro duro", "Mucho contraste", &[(LUM, &[("basicCorrection/saturation", 0.0, 0.0), ("basicCorrection/tone/contrast", 70.0, 70.0)])], None),
    e("Concierto", "Rayos de luz y color", &[("fc:volumetric_rays", &[]), (LUM, &[("basicCorrection/saturation", 130.0, 130.0), ("basicCorrection/tone/contrast", 25.0, 25.0)])], None),
    e("Fugas de luz", "Luz de color que se cuela", &[("fc:light_leaks", &[])], None),
    e("Destellos", "Brillos en forma de estrella", &[("fc:glint", &[])], None),
    e("Caricatura", "Colores planos con contorno", &[("ec.stylize.cartoon", &[])], None),
    e("Acercamiento lento", "Se acerca poco a poco", &[], Some((100.0, 118.0))),
    e("Alejamiento lento", "Se aleja poco a poco", &[], Some((118.0, 100.0))),
    e("Cámara en mano", "Temblor de cámara", &[("fc:camera_shake", &[])], Some((106.0, 106.0))),
    e("Terremoto", "Temblor fuerte", &[("fc:camera_shake", &[]), ("fc:wiggle", &[])], Some((112.0, 112.0))),
    e("Aparecer desenfocado", "Entra borroso y se enfoca", &[("ec.blur.gaussian", &[("blurriness", 90.0, 0.0)])], None),
    e("Noche azul", "Día convertido en noche", &[(LUM, &[("basicCorrection/tone/exposure", -1.1, -1.1), ("basicCorrection/whiteBalance/temperature", -70.0, -70.0), ("basicCorrection/tone/contrast", 25.0, 25.0), ("vignette/vignetteAmount", -2.0, -2.0)])], None),
    e("Eco visual", "Estela de movimiento", &[("ec.time.echo", &[])], None),
    e("Fiesta", "Zoom y colores con cada golpe", &[("bc:ritmo_zoom", &[]), ("bc:ritmo_color", &[]), ("bc:confeti", &[])], None),
    e("Antro", "Destellos, RGB y neón al ritmo", &[("bc:ritmo_destello", &[("fuerza", 45.0, 45.0)]), ("bc:ritmo_rgb", &[]), (LUM, &[("basicCorrection/saturation", 150.0, 150.0), ("basicCorrection/tone/contrast", 30.0, 30.0)])], None),
    e("Golpe seco", "Sacudida y zoom al ritmo", &[("bc:ritmo_sacudida", &[]), ("bc:ritmo_zoom", &[("fuerza", 8.0, 8.0)])], None),
    e("Tele de la abuela", "Pantalla curva con rayas", &[("bc:tv", &[]), (LUM, &[("basicCorrection/saturation", 85.0, 85.0)])], None),
    e("Cinta perdida", "Banda de ruido y temblor", &[("bc:cinta", &[]), ("bc:tv", &[("curva", 15.0, 15.0), ("lineas", 35.0, 35.0)])], None),
    e("Anime de acción", "Líneas de velocidad y contraste", &[("bc:velocidad", &[]), (LUM, &[("basicCorrection/saturation", 130.0, 130.0), ("basicCorrection/tone/contrast", 25.0, 25.0)])], Some((100.0, 112.0))),
    e("Página de manga", "Tinta y trama en blanco y negro", &[("bc:manga", &[])], None),
    e("Historieta", "Puntos de imprenta a color", &[("bc:puntos", &[])], None),
    e("Atardecer morado", "Dos tonos con neblina", &[("bc:duotono", &[("mezcla", 70.0, 70.0)]), ("bc:neblina", &[("cant", 25.0, 25.0)])], None),
    e("Balada", "Corazones, brillo suave y luz cálida", &[("bc:corazones", &[("cant", 60.0, 60.0)]), ("fc:wonder_glow", &[]), (LUM, &[("basicCorrection/whiteBalance/temperature", 25.0, 25.0)])], None),
    e("Bosque mágico", "Luciérnagas entre la niebla", &[("bc:luciernagas", &[]), ("bc:neblina", &[]), (LUM, &[("basicCorrection/tone/exposure", -0.5, -0.5)])], None),
    e("Recuerdo", "Foto instantánea con colores lavados", &[(LUM, &[("creative/adjustments/fadedFilm", 45.0, 45.0), ("basicCorrection/whiteBalance/temperature", 20.0, 20.0)]), ("bc:marco", &[])], None),
    e("Viaje", "Prisma que gira y colores vivos", &[("bc:prisma", &[]), ("bc:agua", &[("cant", 20.0, 20.0)]), (LUM, &[("basicCorrection/saturation", 135.0, 135.0)])], None),
    e("Cine ancho", "Barras que se cierran y tono de película", &[(LUM, &[("basicCorrection/tone/contrast", 20.0, 20.0), ("basicCorrection/saturation", 88.0, 88.0), ("vignette/vignetteAmount", -1.2, -1.2)]), ("bc:barras", &[("abrir", 100.0, 0.0)])], None),
];

/// Pone un estilo en un clip. Devuelve cuántos efectos agregó.
pub fn aplicar(p: &mut Proyecto, clip: u64, i: usize) -> usize {
    let Some(es) = ESTILOS.get(i) else { return 0 };
    let (bpm, inicio) = (p.bpm as f64, p.clip_video(clip).map_or(0.0, |c| c.inicio));
    let mut v = vec![];
    for (tipo, vals) in es.fx {
        let id = p.id();
        let Some(mut f) = fx_nuevo(id, tipo) else { continue };
        if *tipo == LUM {
            // Parte de neutro: el estilo manda.
            for k in ["basicCorrection/tone/contrast", "basicCorrection/tone/exposure"] {
                if let Some(q) = f.p.get_mut(k) {
                    q.v = Valor::Num(0.0);
                }
            }
            if let Some(q) = f.p.get_mut("basicCorrection/saturation") {
                q.v = Valor::Num(100.0);
            }
        }
        crate::vpropios::a_tempo(&mut f, bpm, inicio);
        for (k, a, b) in *vals {
            if let Some(q) = f.p.get_mut(*k) {
                q.v = Valor::Num(*a);
                if a != b {
                    q.claves = vec![(0.0, Valor::Num(*a)), (1.2, Valor::Num(*b))];
                }
            }
        }
        v.push(f);
    }
    let n = v.len();
    if let Some(c) = p.clip_video(clip) {
        c.fx.extend(v);
        if let Some((a, b)) = es.escala {
            c.mov.escala = Prop { v: Valor::Num(a), claves: if a == b { vec![] } else { vec![(0.0, Valor::Num(a)), (c.dur, Valor::Num(b))] }, curvas: vec![] };
        }
    }
    n + usize::from(es.escala.is_some())
}

/// Golpes de un audio: segundos donde la energía sube de golpe, con su fuerza de 0 a 1.
pub fn golpes(a: &crate::medios::Audio, de: f64, hasta: f64) -> Vec<(f64, f64)> {
    let n = 1024;
    let i0 = (de * SR as f64) as usize;
    let i1 = ((hasta * SR as f64) as usize).min(a.cuadros.len());
    if i1 <= i0 + n {
        return vec![];
    }
    let e: Vec<f64> = a.cuadros[i0..i1].chunks(n).map(|c| (c.iter().flatten().map(|x| (*x * *x) as f64).sum::<f64>() / c.len() as f64).sqrt()).collect();
    let d: Vec<f64> = (0..e.len()).map(|i| if i == 0 { 0.0 } else { (e[i] - e[i - 1]).max(0.0) }).collect();
    let max = d.iter().cloned().fold(0.0, f64::max);
    if max <= 0.0 {
        return vec![];
    }
    let mut v: Vec<(f64, f64)> = vec![];
    let paso = n as f64 / SR as f64;
    for i in 1..d.len().saturating_sub(1) {
        // Pico local que además sobresale de su medio segundo.
        let (a, b) = (i.saturating_sub(12), (i + 12).min(d.len()));
        let media = d[a..b].iter().sum::<f64>() / (b - a) as f64;
        if d[i] >= d[i - 1] && d[i] > d[i + 1] && d[i] > 2.2 * media && d[i] > 0.08 * max {
            let t = i as f64 * paso;
            if v.last().is_none_or(|u| t - u.0 > 0.18) {
                v.push((t, (d[i] / max).sqrt()));
            }
        }
    }
    v
}

/// La imagen late con la música: cuadros clave de escala en cada golpe.
pub fn latir(c: &mut ClipVideo, golpes: &[(f64, f64)], fuerza: f64) -> usize {
    let base = c.mov.escala.en(0.0).num();
    let mut k: Vec<(f64, Valor)> = vec![(0.0, Valor::Num(base))];
    for (t, f) in golpes.iter().filter(|g| g.0 > 0.05 && g.0 < c.dur - 0.2) {
        let u = k.last().map_or(0.0, |x| x.0);
        if t - 0.04 > u {
            k.push((t - 0.04, Valor::Num(base)));
        }
        k.push((*t, Valor::Num(base * (1.0 + fuerza * f))));
        k.push((t + 0.14, Valor::Num(base)));
    }
    let n = (k.len() - 1) / 3;
    c.mov.escala.claves = k;
    n
}
