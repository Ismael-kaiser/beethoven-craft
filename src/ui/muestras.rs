//! Miniaturas de efectos, transiciones y estilos sobre el cuadro que se está viendo.
use crate::composicion::{self, Cuadro, Fuente, fx_nuevo};
use crate::modelo::*;
use crate::{estilos, nombres};
use std::sync::Arc;

pub const AN: usize = 128;
pub const AL: usize = 72;

/// Siempre el mismo cuadro.
struct Fija(Arc<Cuadro>);

impl Fuente for Fija {
    fn cuadro(&mut self, _: &ClipVideo, _: f64, _: usize, _: usize) -> Option<Arc<Cuadro>> {
        Some(self.0.clone())
    }
    fn recientes(&self, _: &ClipVideo) -> Vec<Arc<Cuadro>> {
        // Copias corridas, para que los efectos de tiempo tengan algo que mezclar.
        (0..8usize)
            .map(|k| {
                let mut rgba = self.0.rgba.clone();
                rgba.chunks_exact_mut(AN * 4).for_each(|d| d.rotate_left(k * 12));
                Arc::new(Cuadro { t: 1.0 - k as f64 / 24.0, ancho: AN, alto: AL, rgba })
            })
            .collect()
    }
}

/// Reduce un cuadro al tamaño de muestra.
pub fn reducir(an: usize, al: usize, q: &[u8]) -> Vec<u8> {
    let mut o = vec![0u8; AN * AL * 4];
    for y in 0..AL {
        for x in 0..AN {
            let (x0, x1) = (x * an / AN, ((x + 1) * an / AN).max(x * an / AN + 1));
            let (y0, y1) = (y * al / AL, ((y + 1) * al / AL).max(y * al / AL + 1));
            let mut s = [0u32; 3];
            let mut n = 0;
            for yy in (y0..y1.min(al)).step_by(2) {
                for xx in (x0..x1.min(an)).step_by(2) {
                    let p = (yy * an + xx) * 4;
                    (0..3).for_each(|k| s[k] += q[p + k] as u32);
                    n += 1;
                }
            }
            let d = (y * AN + x) * 4;
            (0..3).for_each(|k| o[d + k] = (s[k] / n.max(1)) as u8);
            o[d + 3] = 255;
        }
    }
    o
}

/// Imagen de prueba cuando no hay video.
pub fn de_prueba() -> Vec<u8> {
    let mut o = vec![255u8; AN * AL * 4];
    for y in 0..AL {
        for x in 0..AN {
            let (u, v) = (x as f32 / AN as f32, y as f32 / AL as f32);
            let sol = (((u - 0.7) * 1.78).powi(2) + (v - 0.35).powi(2)).sqrt() < 0.16;
            let monte = v > 0.62 + 0.14 * (u * 9.0).sin() * (u * 4.0).cos();
            let c = if monte { [40.0 + 50.0 * u, 90.0 - 30.0 * v, 70.0] } else if sol { [255.0, 214.0, 120.0] } else { [120.0 + 110.0 * v, 110.0 + 40.0 * v, 190.0 - 70.0 * v] };
            let d = (y * AN + x) * 4;
            (0..3).for_each(|k| o[d + k] = c[k] as u8);
        }
    }
    o
}

fn base() -> Proyecto {
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto, p.fps) = (1920, 1080, 24.0);
    let mut c = p.nuevo_video("/".into(), Contenido::Archivo, 0.0, 2.0);
    (c.ancho, c.alto, c.total) = (1920, 1080, 10.0);
    p.video.push(c);
    p
}

/// Calcula todas las muestras: (clave, RGBA).
pub fn calcular(cuadro: Vec<u8>, entregar: impl Fn(String, Vec<u8>) -> bool) {
    let mut f = Fija(Arc::new(Cuadro { t: 1.0, ancho: AN, alto: AL, rgba: cuadro.clone() }));
    for (i, e) in estilos::ESTILOS.iter().enumerate() {
        let mut p = base();
        let id = p.video[0].id;
        estilos::aplicar(&mut p, id, i);
        // Los que animan se muestran a medio camino.
        let o = composicion::componer(&p, if e.nombre.starts_with("Aparecer") { 0.4 } else { 1.6 }, AN, AL, &mut f);
        if !entregar(format!("estilo:{i}"), o) {
            return;
        }
    }
    for i in 0..crate::filtros::FILTROS.len() {
        let mut p = base();
        p.video[0].fx.extend(crate::filtros::efecto(5, i));
        if !entregar(format!("filtro:{i}"), composicion::componer(&p, 1.0, AN, AL, &mut f)) {
            return;
        }
    }
    for e in nombres::EFECTOS {
        let mut p = base();
        if let Some(x) = fx_nuevo(5, e.2) {
            p.video[0].fx.push(x);
        }
        if !entregar(e.2.to_string(), composicion::componer(&p, 1.0, AN, AL, &mut f)) {
            return;
        }
    }
    for e in nombres::TRANSICIONES {
        // De un color plano al cuadro, a la mitad.
        let mut p = base();
        let mut c = p.nuevo_video("c".into(), Contenido::Color([0.55, 0.16, 0.2, 1.0]), 0.0, 1.0);
        c.capa = 0;
        p.video[0].inicio = 1.0;
        p.video[0].entrada = Some(Transicion { tipo: e.2.into(), dur: 1.0, p: Default::default() });
        p.video.insert(0, c);
        if !entregar(format!("tr:{}", e.2), composicion::componer(&p, 1.45, AN, AL, &mut f)) {
            return;
        }
    }
}
