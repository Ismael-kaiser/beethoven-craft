//! La composición en la tarjeta gráfica: los mismos cuadros que `composicion`, con los efectos como núcleos de cómputo.
//! Lo que no tenga versión para la tarjeta se hace en el procesador y se vuelve a subir.
use crate::composicion::{self, Cuadro, Fuente};
use crate::modelo::*;
use crate::vpropios::{azar, golpe};
use craft_gpu::{Capa, Datos, Formato, FxStep, Gpu, Mesa};
use effectcraft_color::BlendMode;
use effectcraft_effects as ec;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

/// El usuario la apagó (o falló).
static APAGADA: AtomicBool = AtomicBool::new(false);
/// Fallas seguidas: a la tercera se deja de intentar.
static FALLAS: AtomicU32 = AtomicU32::new(0);

pub fn apagar(si: bool) {
    APAGADA.store(si, Relaxed);
}

pub fn encendida() -> bool {
    !APAGADA.load(Relaxed)
}

/// La tarjeta, si hay y sirve. Se abre la primera vez que se usa.
pub fn gpu() -> Option<&'static Gpu> {
    static G: OnceLock<Option<Gpu>> = OnceLock::new();
    if APAGADA.load(Relaxed) || FALLAS.load(Relaxed) >= 3 {
        return None;
    }
    G.get_or_init(|| if std::env::var_os("BC_SIN_GPU").is_some() { None } else { Gpu::headless() }).as_ref()
}

/// Efecto propio listo para la tarjeta.
struct Plan {
    nucleo: &'static str,
    d: Datos,
    lista: Option<Vec<f32>>,
    /// Necesita las luces borrosas: desde qué brillo y con qué radios.
    luces: Option<(f32, f32, f32)>,
    /// Rayos: hacia dónde.
    rayos: Option<(f32, f32)>,
    /// Va otro núcleo después, con su lista.
    luego: Option<(&'static str, Vec<f32>)>,
}

/// Los números de un efecto `bc:…`, igual que los calcula `vpropios::aplicar`. `None`: no tiene versión para la tarjeta.
fn plan(f: &FxVideo, t: f64, w: usize, h: usize, escala: f64) -> Option<Plan> {
    let num = |k: &str| f.p.get(k).map_or(0.0, |p| p.en(t).num());
    let n = |k: &str| num(k) as f32;
    let por = |k: &str| (num(k) / 100.0) as f32;
    let rad = |k: &str| n(k).to_radians();
    let col = |k: &str| match f.p.get(k).map(|p| p.en(t)) {
        Some(Valor::Color(c)) => c,
        _ => [1.0; 4],
    };
    let op = |k: &str| match f.p.get(k).map(|p| p.en(t)) {
        Some(Valor::Op(i)) => i as i32,
        _ => 0,
    };
    let (wf, hf) = (w as f32, h as f32);
    let (cx, cy) = (wf / 2.0, hf / 2.0);
    let e = escala as f32;
    let tv = t as f32 * por("vel");
    let g = golpe(t, num("bpm"), num("cada"), num("desf"), num("caida"));
    let mezcla = if f.p.contains_key("mezcla") { por("mezcla") } else { 1.0 };
    let cua = (t * 24.0).floor() as i32;
    let c1 = col("color");
    let semilla = f.id as i32;
    let mut pl = Plan { nucleo: "", d: Datos::default(), lista: None, luces: None, rayos: None, luego: None };
    pl.d.f[11] = [wf, hf, e, mezcla];
    pl.d.f[10] = c1;
    let mut pon = |nucleo: &'static str, a: &[f32], z: &[i32]| {
        pl.nucleo = nucleo;
        a.iter().enumerate().for_each(|(i, v)| pl.d.f[i / 4][i % 4] = *v);
        z.iter().enumerate().for_each(|(i, v)| pl.d.u[3][i] = *v as u32);
    };
    let particulas = |total: f32, abajo: f32, tam: f32| -> Vec<(f32, f32, f32, i32)> {
        crate::vpropios::particulas(total, abajo, tam, por("cant"), por("tam"), tv, semilla, (wf, hf))
    };
    let arco = crate::vpropios::arcoiris;
    // Partículas: figura, suma y color de cada una.
    let estampa = |ps: Vec<(f32, f32, f32, i32)>, color: &dyn Fn(i32) -> [f32; 4]| -> Vec<f32> { celdas(&ps, color, (wf, hf)) };
    let mut lista = None;
    let mut luces = None;
    match f.tipo.as_str() {
        "bc:color" => {
            let pto = |k: &str| match f.p.get(k).map(|p| p.en(t)) {
                Some(Valor::Punto(v)) => [v[0] as f32, v[1] as f32],
                _ => [0.0; 2],
            };
            let rueda = |k: &str| {
                let [x, y] = pto(k);
                let b = num(&format!("{k}_l")) as f32 / 100.0 * 0.5;
                let e = |g: f32| x * g.to_radians().cos() + y * g.to_radians().sin();
                [e(0.0) * 0.5 + b, e(120.0) * 0.5 + b, e(240.0) * 0.5 + b]
            };
            let (s, m, l) = (rueda("sombras"), rueda("medios"), rueda("luces"));
            let con = n("con") / 100.0;
            let con = if con > 0.0 { 1.0 + con * 1.5 } else { 1.0 + con * 0.8 };
            let mut a = [s, m, l].concat();
            a.extend([2f32.powf(n("exp") / 2.2), con, por("sat"), por("vib"), por("temp") * 0.22, por("tinte") * 0.16, rad("giro"), por("deslavar"), por("vin"), por("grano")]);
            pon("bc_color", &a, &[cua]);
        }
        "bc:ritmo_zoom" => pon("bc_zoom", &[1.0 + por("fuerza") * g.1], &[]),
        "bc:ritmo_destello" => pon("bc_destello", &[por("fuerza") * g.1], &[]),
        "bc:ritmo_color" => pon("bc_tono", &[((g.0 + 1) as f32 * n("paso")).to_radians()], &[]),
        "bc:ritmo_sacudida" => {
            let k = n("fuerza") * e * g.1;
            let tt = t as f32;
            pon("bc_sacudida", &[k * (tt * 71.0).sin(), k * (tt * 93.0 + 1.0).cos(), 1.0 + 2.0 * k / wf.max(1.0)], &[]);
        }
        "bc:ritmo_rgb" => pon("bc_rgb", &[n("fuerza") * e * g.1], &[]),
        "bc:ritmo_vueltas" => pon("bc_vueltas", &[], &[g.0.rem_euclid(4)]),
        "bc:tv" => pon("bc_tv", &[por("curva") * 0.28, por("lineas"), por("vin")], &[]),
        "bc:cinta" => pon("bc_cinta", &[por("cant"), hf * (1.0 - (tv * 0.23).fract() * 1.2)], &[cua]),
        "bc:nocturna" => pon("bc_nocturna", &[por("brillo"), por("ruido")], &[cua]),
        "bc:termica" => pon("bc_termica", &[], &[]),
        "bc:consola" => pon("bc_consola", &[(n("tam") * e).max(1.5)], &[]),
        "bc:barras" => {
            let alto = (wf / n("razon").max(0.5)).min(hf);
            pon("bc_barras", &[(hf - alto) / 2.0 * (1.0 - por("abrir"))], &[]);
        }
        "bc:marco" => {
            let o = por("tam") * hf;
            pon("bc_marco", &[o, o + por("pie") * hf], &[]);
        }
        "bc:puntos" => {
            let (si, co) = rad("ang").sin_cos();
            pon("bc_puntos", &[(n("tam") * e).max(2.0), si, co], &[]);
        }
        "bc:manga" => pon("bc_manga", &[por("tinta"), (n("tam") * e).max(2.0)], &[]),
        "bc:duotono" => pon("bc_duotono", &[], &[]),
        "bc:neblina" => pon("bc_neblina", &[por("cant"), tv], &[]),
        "bc:velocidad" => pon("bc_velocidad", &[por("cant"), por("hueco")], &[(tv * 16.0).floor() as i32, semilla]),
        "bc:prisma" => pon("bc_prisma", &[por("cant") * 0.5 * hf, tv * 0.4], &[num("caras").round().max(2.0) as i32]),
        "bc:tunel" => pon("bc_tunel", &[por("paso").clamp(0.2, 0.95), rad("giro"), (tv * 0.5).rem_euclid(1.0)], &[]),
        "bc:agua" => pon("bc_agua", &[por("cant") * 0.035 * hf, por("tam") * hf, tv], &[]),
        "bc:halo" | "bc:anamorfico" => {
            let ancho = f.tipo == "bc:anamorfico";
            let r = por("tam") * if ancho { wf } else { hf };
            luces = Some((por("desde"), r, if ancho { 2.0 * e } else { r }));
            pon("bc_halo", &[por("cant") * if ancho { 1.9 } else { 1.0 }], &[]);
        }
        "bc:foco" => {
            let r = por("cant") * hf * 0.06;
            luces = Some((-1.0, r, r));
            pon("bc_foco", &[cx + por("x") * wf, cy + por("y") * hf, por("radio"), por("cant")], &[]);
        }
        "bc:vidrio" => {
            let (se, co) = rad("ang").sin_cos();
            pon("bc_vidrio", &[(por("tam") * wf).max(2.0), por("cant"), se, co], &[]);
        }
        "bc:barrido" => {
            let (se, co) = rad("ang").sin_cos();
            let cada = num("cada").max(0.1);
            let u0 = ((t / cada).rem_euclid(1.0) as f32 * 2.6 - 1.3) * (wf * co.abs() + hf * se.abs()) * 0.5;
            pon("bc_barrido", &[se, co, u0, por("ancho") * wf * 0.5, por("cant")], &[]);
        }
        "bc:degradado" => {
            let (se, co) = rad("ang").sin_cos();
            pon("bc_degradado", &[se, co, (wf * co.abs() + hf * se.abs()).max(1.0), por("cant")], &[]);
        }
        "bc:lluvia" => pon("bc_lluvia", &[por("cant"), rad("ang").tan(), tv], &[]),
        "bc:radial" => pon("bc_radial", &[por("cant") * 0.35, cx + por("x") * wf, cy + por("y") * hf], &[]),
        "bc:caleidoscopio" => {
            let paso = std::f32::consts::TAU / (por("caras") * 100.0).round().max(2.0);
            pon("bc_caleidoscopio", &[paso, rad("giro") + tv * 0.35], &[]);
        }
        "bc:latido" => {
            let u = (t * num("bpm") / 60.0).rem_euclid(1.0) as f32;
            let g = |c: f32, a: f32| a * (-((u - c) / 0.055).powi(2)).exp();
            pon("bc_zoom", &[1.0 + por("cant") * 0.35 * (g(0.10, 1.0) + g(0.32, 0.6))], &[]);
        }
        "bc:pendulo" => {
            let (se, co) = (rad("cant") * (tv * 3.0).sin()).sin_cos();
            let m = rad("cant").sin().abs();
            pon("bc_pendulo", &[se, co, 1.0 / (1.0 + m * (wf / hf).max(hf / wf) * 0.95)], &[]);
        }
        "bc:rebote" => {
            let u = (tv * 1.1).rem_euclid(1.0);
            let salto = (4.0 * u * (1.0 - u)) * por("cant") * hf * 0.5;
            pon("bc_rebote", &[1.0 + 0.12 * por("cant") * (-(u.min(1.0 - u) / 0.06).powi(2)).exp(), salto], &[]);
        }
        "bc:remolino" => pon("bc_remolino", &[rad("cant") * if tv != 0.0 { (tv * 0.9).sin() } else { 1.0 }, por("radio") * hf * 0.6], &[]),
        "bc:onda" => {
            let u = (t / num("cada").max(0.1)).rem_euclid(1.0) as f32;
            let largo = (wf * wf + hf * hf).sqrt() * 0.6;
            pon("bc_onda", &[cx + por("x") * wf, cy + por("y") * hf, u * largo, hf * 0.09, por("cant") * hf * 0.06 * (1.0 - u)], &[]);
        }
        "bc:sismo" => {
            let k = por("cant") * hf * 0.02;
            let f = |a: f32, b: f32| ((tv * a).sin() + (tv * b + 1.3).sin() * 0.6) * k;
            pon("bc_sismo", &[f(47.0, 83.0), f(61.0, 97.0), k * 0.35, 1.0 / (1.0 + por("cant") * 0.07)], &[]);
        }
        "bc:azulejos" => pon("bc_azulejos", &[n("caras").round().max(1.0)], &[op("modo")]),
        "bc:espejo" => pon("bc_espejo", &[], &[op("modo")]),
        "bc:giro3d" => pon("bc_giro3d", &[rad("rx"), rad("ry") + rad("vaiven") * (tv * 1.6).sin(), por("zoom").max(0.05)], &[]),
        "bc:cubo" => {
            let (se, co) = (rad("giro") + tv * 0.8).sin_cos();
            pon("bc_cubo", &[se, co], &[]);
        }
        "bc:bandera" => pon("bc_bandera", &[por("cant"), por("tam") * wf, tv], &[]),
        "bc:lente" => pon("bc_lente", &[cx + por("x") * wf, cy + por("y") * hf, por("radio") * hf * 0.5, por("cant")], &[]),
        "bc:gotas" => pon("bc_gotas", &[(por("tam") * hf).max(4.0), por("cant"), tv], &[]),
        "bc:linterna" => {
            let va = por("vaiven");
            pon("bc_linterna", &[por("radio") * hf * 0.6, por("osc"), cx + (tv * 0.9).sin() * va * wf + (tv * 2.3).sin() * wf * 0.01, cy + (tv * 1.3 + 0.7).sin() * va * hf * 0.6], &[]);
        }
        "bc:discoteca" => pon("bc_discoteca", &[por("cant"), n("caras").round().max(2.0), tv], &[]),
        "bc:iris_luz" => {
            let (se, co) = rad("ang").sin_cos();
            pon("bc_iris_luz", &[se, co, por("cant"), tv], &[]);
        }
        "bc:neon" => pon("bc_neon", &[por("cant"), 1.0 - por("osc"), tv], &[op("modo")]),
        "bc:rayos" => {
            luces = Some((por("desde"), 2.0 * e, 2.0 * e));
            pon("bc_rayos", &[por("cant")], &[]);
        }
        "bc:relampago" => {
            let cada = num("cada").max(0.2);
            let (k, u) = ((t / cada).floor() as i32, (t / cada).rem_euclid(1.0) * cada);
            let pulso = |d: f64, a: f32| if u >= d { a * (-((u - d) / 0.07) as f32).exp() } else { 0.0 };
            let g = por("cant") * (pulso(0.0, 1.0) + pulso(0.16 + 0.1 * azar(k, 3) as f64, 0.75));
            pon("bc_relampago", &[g, (0.15 + 0.7 * azar(k, 1)) * wf, hf * (0.55 + 0.4 * azar(k, 6))], &[k]);
        }
        "bc:fugaces" => {
            let v = crate::vpropios::fugaces(por("cant"), tv, wf, hf);
            let mut l = vec![v.len() as f32];
            v.iter().for_each(|q| l.extend([q.0, q.1, q.2, q.3]));
            lista = Some(l);
            pon("bc_fugaces", &[], &[]);
        }
        "bc:fantasmal" => pon("bc_fantasmal", &[por("cant"), 0.9 + 0.12 * (tv * 2.7).sin(), tv], &[]),
        "bc:goteo" => pon("bc_goteo", &[por("cant")], &[]),
        "bc:estatica" => pon("bc_estatica", &[por("cant"), tv], &[(tv * 24.0).floor() as i32]),
        "bc:visor" => pon("bc_visor", &[por("cant")], &[i32::from(t.rem_euclid(1.0) < 0.6)]),
        "bc:rollo" => pon("bc_rollo", &[por("tam") * wf, tv * hf * 0.9], &[]),
        "bc:redondo" => {
            let m = por("orilla") * hf;
            let (hw, hh) = (cx - m, cy - m);
            pon("bc_redondo", &[m, hw, hh, (por("tam") * hf).min(hw).min(hh)], &[]);
        }
        "bc:apertura" => pon("bc_apertura", &[por("abrir") * (cx * cx + cy * cy).sqrt() * 1.02, (por("suave") * hf).max(1.0)], &[i32::from(por("abrir") >= 0.999)]),
        "bc:persianas" => {
            let (se, co) = rad("ang").sin_cos();
            pon("bc_persianas", &[se, co, n("caras").round().max(1.0), por("abrir"), wf * co.abs() + hf * se.abs()], &[]);
        }
        "bc:vhs" => pon("bc_vhs", &[por("cant"), tv], &[(tv * 25.0).floor() as i32]),
        "bc:boceto" => pon("bc_boceto", &[por("cant"), por("color_k")], &[]),
        "bc:tramado" => pon("bc_tramado", &[(n("tam") * e).max(1.0)], &[]),
        "bc:codigo" => pon("bc_codigo", &[(por("tam") * hf).max(4.0), por("cant"), tv], &[]),
        "bc:mosaico" => pon("bc_mosaico", &[(por("tam") * hf).max(3.0), por("junta")], &[]),
        "bc:confeti" => {
            lista = Some(estampa(particulas(140.0, 1.0, 0.018), &|i| arco(azar(i, 21))));
            pon("bc_estampa", &[tv], &[0, 0]);
        }
        "bc:corazones" => {
            lista = Some(estampa(particulas(34.0, -1.0, 0.045), &|i| { let k = 0.75 + 0.25 * azar(i, 51); [c1[0] * k, c1[1] * k, c1[2] * k, 1.0] }));
            pon("bc_estampa", &[tv], &[1, 0]);
        }
        "bc:luciernagas" => {
            lista = Some(estampa(particulas(60.0, -0.35, 0.04), &|_| c1));
            pon("bc_estampa", &[tv], &[2, 1]);
        }
        "bc:bokeh" => {
            lista = Some(estampa(particulas(26.0, -0.35, 0.07), &|i| [c1[0], c1[1] * (0.85 + 0.15 * azar(i, 8)), c1[2] * (0.8 + 0.2 * azar(i, 7)), 1.0]));
            pon("bc_estampa", &[tv], &[3, 1]);
        }
        "bc:brillos" => {
            lista = Some(estampa(particulas(54.0, 0.12, 0.035), &|_| c1));
            pon("bc_estampa", &[tv], &[4, 1]);
        }
        "bc:nieve" => {
            lista = Some(estampa(particulas(160.0, 1.0, 0.008), &|_| [1.0; 4]));
            pon("bc_estampa", &[tv], &[5, 0]);
        }
        "bc:fuegos" => {
            let ps = crate::vpropios::fuegos(por("cant"), por("tam"), tv, wf, hf);
            lista = Some(estampa(ps, &|i| { let c = arco(azar(i / 1000, 5)); [0.35 + 0.65 * c[0], 0.35 + 0.65 * c[1], 0.35 + 0.65 * c[2], 1.0] }));
            pon("bc_estampa", &[tv], &[6, 1]);
        }
        "bc:globos" => {
            lista = Some(estampa(particulas(12.0, -0.9, 0.06), &|i| { let c = arco(azar(i, 21)); [0.25 + 0.75 * c[0], 0.2 + 0.7 * c[1], 0.25 + 0.75 * c[2], 1.0] }));
            pon("bc_estampa", &[tv], &[7, 0]);
        }
        "bc:burbujas" => {
            lista = Some(estampa(particulas(22.0, -0.5, 0.045), &|i| { let c = arco(azar(i, 11) + tv * 0.1); [0.6 + 0.4 * c[0], 0.6 + 0.4 * c[1], 0.6 + 0.4 * c[2], 1.0] }));
            pon("bc_estampa", &[tv], &[8, 1]);
        }
        "bc:petalos" => {
            lista = Some(estampa(particulas(46.0, 0.75, 0.016), &|i| { let k = 0.8 + 0.3 * azar(i, 4); [c1[0] * k, c1[1] * k, c1[2] * k, 1.0] }));
            pon("bc_estampa", &[tv], &[9, 0]);
        }
        "bc:murcielagos" => {
            let ps = crate::vpropios::murcielagos(por("cant"), por("tam"), tv, wf, hf);
            lista = Some(estampa(ps, &|_| [0.02, 0.01, 0.03, 1.0]));
            pon("bc_estampa", &[tv], &[10, 0]);
        }
        "bc:polvo" => {
            let (motas, rayas) = crate::vpropios::polvo(por("cant"), t, e, wf, hf);
            lista = Some(estampa(motas, &|i| if azar(i, 41) > 0.5 { [0.96, 0.94, 0.88, 1.0] } else { [0.05, 0.04, 0.03, 1.0] }));
            pon("bc_estampa", &[tv], &[11, 0]);
            let mut l = vec![rayas.len() as f32];
            rayas.iter().for_each(|q| l.extend([q.0, q.1, q.2]));
            pl.luego = Some(("bc_polvo2", l));
        }
        _ => return None,
    }
    (pl.lista, pl.luces) = (lista, luces);
    if f.tipo == "bc:rayos" {
        pl.rayos = Some((cx + por("x") * wf, cy + por("y") * hf));
    }
    // Los colores de las mezclas de dos tonos.
    if matches!(f.tipo.as_str(), "bc:duotono" | "bc:degradado" | "bc:tramado") {
        (pl.d.f[8], pl.d.f[9]) = (col("osc"), col("cla"));
    }
    Some(pl)
}

/// Las partículas repartidas en una cuadrícula, para que cada pixel solo vea las de su celda.
fn celdas(ps: &[(f32, f32, f32, i32)], color: &dyn Fn(i32) -> [f32; 4], (wf, hf): (f32, f32)) -> Vec<f32> {
    const C: usize = 48;
    const F: usize = 27;
    let mut celda: Vec<Vec<f32>> = vec![vec![]; C * F];
    for (k, (x, y, r, _)) in ps.iter().enumerate() {
        let r = r + 1.0;
        let rango = |a: f32, b: f32, n: usize| (a * n as f32).floor().clamp(0.0, n as f32) as usize..(b * n as f32).floor().clamp(-1.0, n as f32 - 1.0) as i32 as isize as usize + 1;
        if x + r < 0.0 || y + r < 0.0 || x - r > wf || y - r > hf {
            continue;
        }
        for j in rango((y - r) / hf, ((y + r) / hf).max(0.0), F) {
            for i in rango((x - r) / wf, ((x + r) / wf).max(0.0), C) {
                celda[j * C + i].push(k as f32);
            }
        }
    }
    let mut v = vec![C as f32, F as f32];
    let mut n = 0;
    for c in &celda {
        v.push(n as f32);
        n += c.len();
    }
    v.push(n as f32);
    celda.iter().for_each(|c| v.extend(c));
    for (x, y, r, i) in ps {
        let c = color(*i);
        v.extend([*x, *y, *r, *i as f32, c[0], c[1], c[2], 0.0]);
    }
    v
}

fn propio(m: &mut Mesa, c: &Capa, f: &FxVideo, t: f64) -> Option<Capa> {
    let (w, h) = (c.ancho() as usize, c.alto() as usize);
    let p = plan(f, t, w, h, c.scale)?;
    let aux = match p.luces {
        Some((desde, rx, ry)) => {
            let mut d = Datos::default();
            d.f[0][0] = desde;
            d.f[11] = [w as f32, h as f32, c.scale as f32, 1.0];
            let chica = ((w as u32 / 4).max(1), (h as u32 / 4).max(1));
            let l = m.propio("bc_luces", c, &d, None, None, Some(chica))?;
            let r = ((rx / 4.0) as usize, (ry / 4.0) as usize);
            let l = m.caja(&l, &[r, r]);
            Some(match p.rayos {
                Some((x, y)) => {
                    d.f[0] = [0.0, x, y, 0.0];
                    let l = m.propio("bc_rayos_paso", &l, &d, None, None, None)?;
                    m.caja(&l, &[(1, 1)])
                }
                None => l,
            })
        }
        None => None,
    };
    let o = m.propio(p.nucleo, c, &p.d, aux.as_ref(), p.lista.as_deref(), None)?;
    match p.luego {
        Some((n, l)) => m.propio(n, &o, &p.d, None, Some(&l), None),
        None => Some(o),
    }
}

/// Efectos de FilmCraft con versión para la tarjeta.
const FC: [&str; 16] = ["wonder_glow", "rotate_3d", "grow", "shrink", "move", "spin", "wiggle", "camera_shake", "rounded_crop", "clone", "echo_glow", "edge_glow", "glint", "light_leaks", "rgb_split", "volumetric_rays"];

/// Un efecto de FilmCraft: en luz lineal, como en `craft_trans::vfx`.
fn de_fc(m: &mut Mesa, c: &Capa, f: &FxVideo, t: f64, dur: f64, fps: f64, tam: [f64; 2]) -> Option<Capa> {
    use craft_trans::vfx::{bv, chv, cv, fbm, fv, lin, placements, pv_in};
    let id = f.tipo.strip_prefix("fc:")?;
    if !FC.contains(&id) {
        return None;
    }
    let e = composicion::valores_fc(f, craft_trans::effect::find_video(id)?, t, tam);
    let env = composicion::Entorno { dur, fps };
    let cx = &craft_trans::vfx::FxCtx { t: craft_trans::Tick, px_scale: c.scale as f32, seconds: t, env: Some(&env), frame_rate: fps };
    let (w, h) = (c.ancho() as usize, c.alto() as usize);
    let (wf, hf) = (w as f32, h as f32);
    let datos = |a: &[f32], z: &[i32]| {
        let mut d = Datos::default();
        a.iter().enumerate().for_each(|(i, v)| d.f[i / 4][i % 4] = *v);
        z.iter().enumerate().for_each(|(i, v)| d.u[3][i] = *v as u32);
        d.f[11] = [wf, hf, c.scale as f32, 1.0];
        d
    };
    let v = |k: &str| fv(&e, k, cx);
    let k = cx.px_scale;
    let col = |id: &str| lin(cv(&e, id, cx));
    let o = m.propio("bc_lin", c, &datos(&[], &[]), None, None, None)?;
    let listo = |m: &mut Mesa, o: &Capa| m.propio("bc_srgb", o, &datos(&[], &[]), None, None, None);
    if let Some(p) = placements(&e, cx, w, h) {
        let mut a = vec![];
        for (ce, off, s, rot) in &p {
            // Del punto de la pantalla al de la capa.
            let (si, co) = rot.to_radians().sin_cos();
            let (tx, ty) = (ce.x + off.x, ce.y + off.y);
            let (ia, ib) = (co / s, si / s);
            a.extend([ia, ib, ce.x - ia * tx - ib * ty, -ib, ia, ce.y + ib * tx - ia * ty].map(|v| v as f32));
        }
        let o = m.propio("bc_fc_afin", &o, &datos(&a, &[p.len() as i32]), None, None, None)?;
        return listo(m, &o);
    }
    // Las luces por encima de un umbral, y sumarlas al final.
    let claros = |m: &mut Mesa, soft: f32| m.propio("bc_fc_claros", &o, &datos(&[v("threshold") / 100.0, soft], &[]), None, None, None);
    let luz = |m: &mut Mesa, base: &Capa, l: &Capa, g: [f32; 3], pantalla: bool| m.propio("bc_fc_luz", base, &datos(&g, &[i32::from(pantalla)]), Some(l), None, None);
    let suma = |m: &mut Mesa, a: &Capa, ka: f32, b: &Capa, kb: f32| m.propio("bc_fc_suma", a, &datos(&[ka, kb], &[]), Some(b), None, None);
    let tinte = |m: &mut Mesa, a: &Capa, c: [f32; 3], q: f32, sat: f32| if q > 0.0 || sat != 1.0 { m.propio("bc_fc_tinte", a, &datos(&[c[0], c[1], c[2], q, sat], &[]), None, None, None) } else { Some(a.clone()) };
    let r = match id {
        "rotate_3d" => {
            let (ax, ay, az) = ((v("rot_x") as f64).to_radians(), (v("rot_y") as f64).to_radians(), (v("rot_z") as f64).to_radians());
            let z = v("z") as f64 * k as f64;
            if ax.abs() < 1e-9 && ay.abs() < 1e-9 && az.abs() < 1e-9 && z.abs() < 1e-9 {
                return Some(c.clone());
            }
            let persp = v("perspective").clamp(0.0, 100.0) as f64;
            let ff = if persp <= 0.0 { 1e7 } else { (w.max(h) as f64) * (0.6 + (100.0 - persp) / 100.0 * 4.0) };
            let ((sx, cr), (sy, cy), (sz, cz)) = (ax.sin_cos(), ay.sin_cos(), az.sin_cos());
            // Columnas de Rz · Ry · Rx.
            let u = [cz * cy, sz * cy, -sy];
            let w2 = [cz * sy * sx - sz * cr, sz * sy * sx + cz * cr, cy * sx];
            let n = [cz * sy * cr + sz * sx, sz * sy * cr - cz * sx, cy * cr];
            if bv(&e, "hide_back") && n[2] < 0.0 {
                return m.vacia(w as u32, h as u32, c.scale);
            }
            let a: Vec<f32> = u.iter().chain(&[0.0]).chain(&w2).chain(&[0.0]).chain(&n).chain(&[ff, ff + z]).map(|v| *v as f32).collect();
            m.propio("bc_fc_rotar", &o, &datos(&a, &[]), None, None, None)?
        }
        "rounded_crop" => {
            let (l, t, r, b) = (v("left") / 100.0 * wf, v("top") / 100.0 * hf, wf - v("right") / 100.0 * wf, hf - v("bottom") / 100.0 * hf);
            let rad = (v("radius") * k).min(((r - l).min(b - t) / 2.0).max(0.0));
            let bc = col("border_color");
            m.propio("bc_fc_recorte", &o, &datos(&[l, t, r, b, rad, v("feather") * k, v("border") * k, 0.0, bc[0], bc[1], bc[2]], &[]), None, None, None)?
        }
        "clone" => {
            let (cols, rows) = (v("columns").clamp(1.0, 16.0).floor(), v("rows").clamp(1.0, 16.0).floor());
            if cols == 1.0 && rows == 1.0 {
                return Some(c.clone());
            }
            let gap = v("gap") * k;
            let tw = ((wf - gap * (cols - 1.0)) / cols).max(1.0);
            let th = ((hf - gap * (rows - 1.0)) / rows).max(1.0);
            m.propio("bc_fc_clones", &o, &datos(&[cols, rows, gap, tw, th, (tw / wf).min(th / hf)], &[i32::from(bv(&e, "mirror"))]), None, None, None)?
        }
        "rgb_split" => {
            let amt = v("amount") * k;
            let ang = v("angle").to_radians();
            let p = pv_in(&e, "center", cx, w, h);
            m.propio("bc_fc_rgb", &o, &datos(&[amt, ang.cos(), ang.sin(), v("blend") / 100.0, p.x as f32, p.y as f32, wf.hypot(hf) / 2.0], &[chv(&e, "mode") as i32]), None, None, None)?
        }
        "light_leaks" => {
            // La mancha de luz es suave: se calcula en chico.
            let (c1, c2) = (col("c1"), col("c2"));
            let scale = (v("scale") / 100.0).max(0.05) * wf.max(hf) * 0.6;
            let dir = v("direction").to_radians();
            let tt = (t * v("speed") as f64) as f32;
            let seed = v("seed").max(0.0) as u64;
            let inten = v("intensity") / 100.0;
            let (dx, dy) = (dir.cos(), dir.sin());
            let (cw, ch) = (160usize, (160 * h / w.max(1)).clamp(8, 400));
            let mut img = effectcraft_raster::Image::new(cw as u32, ch as u32);
            use rayon::prelude::*;
            img.data.par_iter_mut().enumerate().for_each(|(i, q)| {
                let (x, y) = (((i % cw) as f32 + 0.5) / cw as f32 * wf, ((i / cw) as f32 + 0.5) / ch as f32 * hf);
                let (u, s) = (x / scale, y / scale);
                let n = fbm(u + dx * tt * 0.3, s + dy * tt * 0.3, tt * 0.15, 3.0, seed) * 0.5 + 0.5;
                let b = fbm(u * 0.7 + 5.2, s * 0.7, tt * 0.1 + 9.0, 2.0, seed) * 0.5 + 0.5;
                *q = [n, b, 0.0, 1.0];
            });
            let aux = m.subir(&ec::Buf { img, offset: [0.0; 2], scale: 1.0 })?;
            m.propio("bc_fc_fugas", &o, &datos(&[c1[0], c1[1], c1[2], inten, c2[0], c2[1], c2[2], 0.0, dx, dy, cw as f32 / wf, ch as f32 / hf], &[chv(&e, "mode") as i32]), Some(&aux), None, None)?
        }
        "volumetric_rays" => {
            let p = pv_in(&e, "center", cx, w, h);
            let mut hi = claros(m, 0.1)?;
            let mut down = 1.0f32;
            while hi.ancho() > 960 {
                hi = m.mitad(&hi);
                down *= 2.0;
            }
            let hi = m.propio("bc_fc_haces", &hi, &datos(&[p.x as f32 / down, p.y as f32 / down, v("length") / 100.0], &[]), None, None, None)?;
            let rayos = if down > 1.0 { m.propio("bc_estirar", &hi, &datos(&[hi.ancho() as f32 / wf], &[]), None, None, Some((w as u32, h as u32)))? } else { hi };
            let base = if bv(&e, "only") { m.vacia(w as u32, h as u32, c.scale)? } else { o.clone() };
            let g = v("intensity") / 100.0 * 2.5;
            luz(m, &base, &rayos, col("color").map(|x| x * g), false)?
        }
        "wonder_glow" => {
            let hi = claros(m, 0.15)?;
            let hi = tinte(m, &hi, col("color"), f32::from(bv(&e, "use_color")), v("saturation") / 100.0)?;
            let r = (v("radius") * k * 0.5) as f64;
            let mut acc: Option<Capa> = None;
            for (q, peso) in [(0.25f64, 0.45f32), (1.0, 0.35), (3.0, 0.2)] {
                let b = m.borroso(&hi, r * q);
                acc = Some(match acc {
                    None => suma(m, &b, peso, &b, 0.0)?,
                    Some(a) => suma(m, &a, 1.0, &b, peso)?,
                });
            }
            luz(m, &o, &acc?, [v("intensity") / 100.0 * 1.5; 3], chv(&e, "mode") == 1)?
        }
        "echo_glow" => {
            let hi = claros(m, 0.1)?;
            let n = v("echoes").clamp(1.0, 12.0) as i32;
            let (r, spread, decay) = (v("radius") * k * 0.5, v("spread").max(1.0), 1.0 - v("decay") / 100.0);
            let mut acc: Option<Capa> = None;
            let mut total = 0.0;
            for i in 0..n {
                let peso = decay.powi(i);
                total += peso;
                let b = m.borroso(&hi, (r * spread.powi(i)) as f64);
                acc = Some(match acc {
                    None => suma(m, &b, peso, &b, 0.0)?,
                    Some(a) => suma(m, &a, 1.0, &b, peso)?,
                });
            }
            let acc = tinte(m, &acc?, col("color"), 1.0, 1.0)?;
            luz(m, &o, &acc, [v("intensity") / 100.0 / total.max(1e-6) * 2.0; 3], false)?
        }
        "edge_glow" => {
            let (th, ancho) = (v("threshold") / 100.0, v("width") * k);
            let mut b = m.propio("bc_fc_bordes", &o, &datos(&[th], &[]), None, None, None)?;
            if ancho > 1.0 {
                b = m.borroso(&b, ancho as f64 * 0.5);
                b = m.propio("bc_fc_nivel", &b, &datos(&[3.0], &[]), None, None, None)?;
            }
            let halo = m.borroso(&b, (v("radius") * k * 0.5) as f64);
            let l = suma(m, &b, 0.6, &halo, 1.6)?;
            let base = if bv(&e, "only") { m.vacia(w as u32, h as u32, c.scale)? } else { o.clone() };
            let q = v("intensity") / 100.0;
            luz(m, &base, &l, col("color").map(|x| x * q), false)?
        }
        "glint" => {
            // Las estelas, a la mitad de tamaño.
            let hi = claros(m, 0.08)?;
            let hi = m.mitad(&hi);
            let rayos = [2usize, 4, 6, 8][chv(&e, "rays").min(3) as usize];
            let len = v("length") * k / 2.0;
            let alcance = len.clamp(0.0, (hi.ancho() + hi.alto()) as f32) * 0.35;
            let len = len * 0.35;
            let a = (-1.0 / len.max(1.0)).exp();
            let rot = v("rotation").to_radians();
            let mut acc: Option<Capa> = None;
            for i in 0..rayos {
                let g = rot + std::f32::consts::TAU * i as f32 / rayos as f32;
                let mut s = hi.clone();
                let mut n = 1.0f32;
                while n < alcance * 3.0 {
                    s = m.propio("bc_fc_estela", &s, &datos(&[g.cos() * n, g.sin() * n, a.powf(n)], &[]), None, None, None)?;
                    n *= 2.0;
                }
                acc = Some(match acc {
                    None => suma(m, &s, 1.0 - a, &s, 0.0)?,
                    Some(x) => suma(m, &x, 1.0, &s, 1.0 - a)?,
                });
            }
            let cc = col("color");
            let acc = tinte(m, &acc?, cc, v("colorize") / 100.0, 1.0)?;
            let acc = m.propio("bc_estirar", &acc, &datos(&[0.5], &[]), None, None, Some((w as u32, h as u32)))?;
            let g = v("intensity") / 100.0 * 3.0;
            luz(m, &o, &acc, cc.map(|x| g * x.max(0.2)), false)?
        }
        _ => return None,
    };
    listo(m, &r)
}

/// ¿Ese efecto corre en la tarjeta?
/// Las transiciones que se hacen en la tarjeta (`bc_transicion`, las mismas cuentas que `craft_trans`): su número.
fn transicion_en_tarjeta(tipo: &str) -> Option<u32> {
    Some(match tipo {
        "cross_dissolve" => 0,
        "additive_dissolve" => 1,
        "film_dissolve" => 2,
        "dip_to_black" => 3,
        "dip_to_white" => 4,
        _ => return None,
    })
}

/// Transición de `a` a `b` con avance `p` (0 a 1), en la tarjeta.
fn mezclar_transicion(m: &mut Mesa, tr: &Transicion, a: &Capa, b: &Capa, p: f64) -> Option<Capa> {
    let mut d = Datos::default();
    d.u[0][0] = transicion_en_tarjeta(&tr.tipo)?;
    d.f[0][0] = p.clamp(0.0, 1.0) as f32;
    m.propio("bc_transicion", a, &d, Some(b), None, None)
}

pub fn en_tarjeta(f: &FxVideo) -> bool {
    if let Some(id) = f.tipo.strip_prefix("fc:") {
        return FC.contains(&id);
    }
    if f.tipo.starts_with("bc:") { plan(f, 0.0, 16, 16, 1.0).is_some() } else { Gpu::soporta(&f.tipo) }
}

#[allow(clippy::too_many_arguments)]
fn aplicar(m: &mut Mesa, c: Capa, f: &FxVideo, t: f64, dur: f64, fps: f64, tam: [f64; 2], pasado: Option<&composicion::Pasado>) -> Option<Capa> {
    if f.tipo.starts_with("bc:") {
        if let Some(o) = propio(m, &c, f, t) {
            return Some(o);
        }
    } else if f.tipo.starts_with("fc:") {
        if let Some(o) = de_fc(m, &c, f, t, dur, fps, tam) {
            return Some(o);
        }
    } else if Gpu::soporta(&f.tipo) {
        if let Some(s) = ec::find(&f.tipo) {
            let params = composicion::valores_ec(f, s, t, tam);
            let env = ec::EffectEnv { comp_time: t, frame_rate: fps, host: pasado.map(|p| p as &dyn ec::EffectHost), ..Default::default() };
            let ctx = ec::EffectCtx { params: &params, time: t, layer_size: tam, seed: f.id as u32, adjustment: false, env };
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| m.efecto(&c, &FxStep { spec: s, ctx })));
            if let Ok(Some(o)) = r {
                return Some(o);
            }
        }
    }
    let b = m.bajar(&c)?;
    let b = composicion::aplicar_fx(f, t, dur, fps, b, tam, pasado);
    m.subir(&b)
}

fn subir_cuadro(m: &mut Mesa, q: &Cuadro, (w, h): (usize, usize), escala: f64) -> Option<Capa> {
    let (datos, formato) = if q.yuv.is_empty() { (&q.rgba, Formato::Rgba) } else { (&q.yuv, Formato::I420) };
    m.subir_video(datos, q.ancho as u32, q.alto as u32, formato, (w as u32, h as u32), escala)
}

fn ajuste(m: &mut Mesa, c: &Capa, op: f32, opaco: bool, r: Option<[f32; 4]>) -> Option<Capa> {
    let mut d = Datos::default();
    d.f[0][0] = op;
    d.f[1] = r.unwrap_or([0.0, 0.0, 1e9, 1e9]);
    d.u[3][0] = u32::from(opaco);
    m.propio("bc_ajuste", c, &d, None, None, None)
}

/// Lo que da `dibujar_clip`, en la tarjeta: la capa con efectos y la matriz con que va sobre el lienzo.
struct Dibujo {
    c: Capa,
    m: [f64; 6],
    op: f32,
    /// Ya está en su sitio y del tamaño del lienzo.
    directo: bool,
}

#[allow(clippy::too_many_arguments)]
fn dibujar(m: &mut Mesa, c: &ClipVideo, t: f64, p: &Proyecto, w: usize, h: usize, f: &mut dyn Fuente) -> Option<Option<Dibujo>> {
    let tl = t - c.inicio;
    let k = h as f64 / p.alto.max(1) as f64;
    let mut b = match &c.contenido {
        Contenido::Archivo | Contenido::Imagen => {
            let Some(q) = composicion::pedido(c, t, w, h).and_then(|(ts, aw, ah)| f.cuadro_libre(c, ts, aw, ah).map(|q| (q, aw, ah))) else { return Some(None) };
            subir_cuadro(m, &q.0, (q.1, q.2), k)?
        }
        Contenido::Color(q) => {
            let v = m.vacia(w as u32, h as u32, k)?;
            let mut d = Datos::default();
            d.f[0] = [q[0] * q[3], q[1] * q[3], q[2] * q[3], q[3]];
            m.propio("bc_color_solido", &v, &d, None, None, None)?
        }
        _ => {
            let Some(b) = composicion::imagen_de(c, t, p, w, h, f) else { return Some(None) };
            m.subir(&b)?
        }
    };
    let (bw, bh) = (b.ancho() as f64, b.alto() as f64);
    let r = c.mov.recorte;
    if r.iter().any(|v| *v > 0.0) {
        let (iw, ih) = (bw as usize, bh as usize);
        let x0 = (iw as f32 * r[0] / 100.0) as usize;
        let y0 = (ih as f32 * r[1] / 100.0) as usize;
        let x1 = iw - ((iw as f32 * r[2] / 100.0) as usize).min(iw);
        let y1 = ih - ((ih as f32 * r[3] / 100.0) as usize).min(ih);
        b = ajuste(m, &b, 1.0, false, Some([x0 as f32, y0 as f32, x1 as f32, y1 as f32]))?;
    }
    let tam = [bw / b.scale, bh / b.scale];
    let pasado = composicion::pasado_de(c, f, b.scale);
    for x in c.fx.iter().filter(|x| x.activo) {
        b = aplicar(m, b, x, tl, c.dur, p.fps, tam, pasado.as_ref())?;
    }
    if let Some(g) = c.nodos.as_ref().filter(|g| g.activo) {
        let z = m.bajar(&b)?;
        let z = crate::nodos::evaluar(g, z, tl, &|f, b| composicion::aplicar_fx(f, tl, c.dur, p.fps, b, tam, pasado.as_ref()));
        b = m.subir(&z)?;
    }
    let mv = &c.mov;
    let esc = (mv.escala.en(tl).num() / 100.0).max(0.0001);
    let giro = mv.giro.en(tl).num().to_radians();
    let (ax, ay) = (mv.ancla[0] / 100.0 * bw, mv.ancla[1] / 100.0 * bh);
    let cx = w as f64 / 2.0 + mv.x.en(tl).num() / 100.0 * w as f64 + ax;
    let cy = h as f64 / 2.0 + mv.y.en(tl).num() / 100.0 * h as f64 + ay;
    let (ox, oy) = (bw / 2.0 + b.offset[0] + ax, bh / 2.0 + b.offset[1] + ay);
    let firme = |v: f64| if v.abs() < 0.0001 { 0.0001 } else { v };
    let (ex, ey) = (firme(esc * (mv.ancho.en(tl).num() / 100.0)), firme(esc * (mv.alto.en(tl).num() / 100.0)));
    let op = (mv.opacidad.en(tl).num() / 100.0).clamp(0.0, 1.0) as f32;
    let directo = ex == 1.0 && ey == 1.0 && giro == 0.0 && ox == cx && oy == cy && (b.ancho() as usize, b.alto() as usize) == (w, h);
    // Punto de la capa → punto del lienzo: escala, gira y pone el apoyo en su lugar.
    let (s, co) = giro.sin_cos();
    let mat = [ex * co, -ey * s, cx - (ex * co * ox - ey * s * oy), ex * s, ey * co, cy - (ex * s * ox + ey * co * oy)];
    Some(Some(Dibujo { c: b, m: mat, op, directo }))
}

/// En un lienzo transparente de `w` × `h`.
fn suelto(m: &mut Mesa, d: Option<Dibujo>, w: usize, h: usize, k: f64) -> Option<Capa> {
    let Some(d) = d else { return m.vacia(w as u32, h as u32, k) };
    if d.directo {
        return if d.op < 1.0 { ajuste(m, &d.c, d.op, false, None) } else { Some(d.c) };
    }
    let v = m.vacia(w as u32, h as u32, k)?;
    Some(m.mover(&v, &d.c, d.m, BlendMode::Normal, d.op))
}

/// Cómo se quiere el cuadro terminado.
#[derive(Clone, Copy, PartialEq)]
pub enum Salida {
    Rgba,
    /// Planos 4:2:0 listos para el codificador.
    I420,
}

/// ¿Conviene la tarjeta para este segundo? Si todo lo pesado va de cualquier modo en el procesador, no.
fn conviene(p: &Proyecto, t: f64) -> bool {
    let mut si = false;
    let mut no = false;
    for (i, capa) in p.capas.iter().enumerate().filter(|x| x.1.visible) {
        let _ = capa;
        let Some(c) = p.video_en(i, t) else { continue };
        for x in c.fx.iter().filter(|x| x.activo) {
            if en_tarjeta(x) { si = true } else { no = true }
        }
        no |= c.nodos.as_ref().is_some_and(|g| g.activo);
        si |= i > 0 || c.mov != Default::default();
    }
    si || !no
}

/// El cuadro de `t` hecho en la tarjeta. `None`: hay que hacerlo en el procesador.
pub fn componer(p: &Proyecto, t: f64, w: usize, h: usize, f: &mut dyn Fuente, salida: Salida) -> Option<Vec<u8>> {
    // Las miniaturas salen más rápido en el procesador.
    if w * h < 60_000 {
        return None;
    }
    let g = gpu()?;
    if !conviene(p, t) {
        return None;
    }
    if salida == Salida::I420 && !(w.is_multiple_of(8) && h.is_multiple_of(2)) {
        return None;
    }
    let r = g.within_memory(|| cuadro(g, p, t, w, h, f, salida));
    g.fin_de_cuadro();
    match r {
        Ok(v) => {
            if v.is_some() {
                FALLAS.store(0, Relaxed);
            }
            v
        }
        Err(_) => {
            FALLAS.fetch_add(1, Relaxed);
            None
        }
    }
}

fn cuadro(g: &Gpu, p: &Proyecto, t: f64, w: usize, h: usize, f: &mut dyn Fuente, salida: Salida) -> Option<Vec<u8>> {
    let m = &mut g.mesa()?;
    let k = h as f64 / p.alto.max(1) as f64;
    let mut lienzo: Option<Capa> = None;
    for (i, capa) in p.capas.iter().enumerate() {
        if !capa.visible {
            continue;
        }
        let Some(c) = p.video_en(i, t) else { continue };
        let modo = composicion::FUSIONES.get(c.mov.fusion as usize).map_or(BlendMode::Normal, |x| x.1);
        if c.contenido == Contenido::Ajuste {
            let Some(l) = lienzo.clone() else { continue };
            let tl = t - c.inicio;
            let tam = [p.ancho as f64, p.alto as f64];
            let mut b = l.clone();
            for x in c.fx.iter().filter(|x| x.activo) {
                b = aplicar(m, b, x, tl, c.dur, p.fps, tam, None)?;
            }
            if let Some(g) = c.nodos.as_ref().filter(|g| g.activo) {
                let z = m.bajar(&b)?;
                let z = crate::nodos::evaluar(g, z, tl, &|f, b| composicion::aplicar_fx(f, tl, c.dur, p.fps, b, tam, None));
                b = m.subir(&z)?;
            }
            if b.offset != [0.0; 2] || (b.ancho() as usize, b.alto() as usize) != (w, h) {
                b = m.recorte(&b, b.offset[0].round() as i64, b.offset[1].round() as i64, w as u32, h as u32);
            }
            let r = c.mov.recorte;
            if r.iter().any(|v| *v > 0.0) {
                let (x0, y0) = ((w as f32 * r[0] / 100.0) as usize, (h as f32 * r[1] / 100.0) as usize);
                let (x1, y1) = ((w as f32 * (1.0 - r[2] / 100.0)) as usize, (h as f32 * (1.0 - r[3] / 100.0)) as usize);
                b = ajuste(m, &b, 1.0, false, Some([x0 as f32, y0 as f32, x1 as f32, y1 as f32]))?;
            }
            let op = (c.mov.opacidad.en(tl).num() / 100.0).clamp(0.0, 1.0) as f32;
            lienzo = Some(m.mezclar(&l, &b, modo, op));
            continue;
        }
        let entra = c.entrada.as_ref().filter(|x| t < c.inicio + x.dur);
        let sale = c.salida.as_ref().filter(|x| t > c.fin() - x.dur);
        // Textos y figuras quietos: el mismo dibujo guardado que usa el procesador.
        if modo == BlendMode::Normal && lienzo.is_some() && composicion::quieto(c, t) {
            let q = composicion::guardado(c, t, p, w, h, f);
            if q.img.width > 0 {
                let s = m.subir_fijo(&q.img)?;
                let l = lienzo.take()?;
                lienzo = Some(m.mover(&l, &s, [1.0, 0.0, q.x as f64, 0.0, 1.0, q.y as f64], BlendMode::Normal, 1.0));
            }
            continue;
        }
        let d = dibujar(m, c, t, p, w, h, f)?;
        let en_tarjeta = |x: Option<&Transicion>| x.is_none_or(|x| transicion_en_tarjeta(&x.tipo).is_some());
        let img = if (entra.is_some() || sale.is_some()) && en_tarjeta(entra) && en_tarjeta(sale) {
            // Disolvencias y pasar por un color: las dos imágenes se mezclan sin salir de la tarjeta.
            let mut b = suelto(m, d, w, h, k)?;
            if let Some(tr) = entra {
                let a = match composicion::previo(p, c) {
                    Some(q) => {
                        let d = dibujar(m, q, t, p, w, h, f)?;
                        suelto(m, d, w, h, k)?
                    }
                    None => m.vacia(w as u32, h as u32, k)?,
                };
                b = mezclar_transicion(m, tr, &a, &b, (t - c.inicio) / tr.dur.max(0.01))?;
            }
            if let Some(tr) = sale {
                let v = m.vacia(w as u32, h as u32, k)?;
                b = mezclar_transicion(m, tr, &b, &v, (t - (c.fin() - tr.dur)) / tr.dur.max(0.01))?;
            }
            Some(Dibujo { c: b, m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0], op: 1.0, directo: true })
        } else if entra.is_some() || sale.is_some() {
            // Las transiciones mezclan dos imágenes completas.
            let b = suelto(m, d, w, h, k)?;
            let mut b = m.bajar(&b)?.img;
            let vacio = || effectcraft_raster::Image::new(w as u32, h as u32);
            if let Some(tr) = entra {
                let a = match composicion::previo(p, c) {
                    Some(q) => {
                        let d = dibujar(m, q, t, p, w, h, f)?;
                        let a = suelto(m, d, w, h, k)?;
                        m.bajar(&a)?.img
                    }
                    None => vacio(),
                };
                b = composicion::transicion(tr, &a, &b, ((t - c.inicio) / tr.dur.max(0.01)) as f32, k as f32);
            }
            if let Some(tr) = sale {
                b = composicion::transicion(tr, &b, &vacio(), ((t - (c.fin() - tr.dur)) / tr.dur.max(0.01)) as f32, k as f32);
            }
            let c = m.subir(&ec::Buf { img: b, offset: [0.0; 2], scale: k })?;
            Some(Dibujo { c, m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0], op: 1.0, directo: true })
        } else {
            d
        };
        let Some(d) = img else { continue };
        // Lo de hasta abajo va sobre negro.
        let l = match lienzo.take() {
            Some(l) => l,
            None if d.directo && modo == BlendMode::Normal => {
                // Si no hay más encima, la salida lo pone sobre negro; si hay, se hace opaco antes.
                let mas = p.capas.iter().enumerate().skip(i + 1).any(|(j, x)| x.visible && p.video_en(j, t).is_some());
                lienzo = Some(if mas || d.op < 1.0 { ajuste(m, &d.c, d.op, true, None)? } else { d.c });
                continue;
            }
            None => {
                let v = m.vacia(w as u32, h as u32, k)?;
                let mut n = Datos::default();
                n.f[0] = [0.0, 0.0, 0.0, 1.0];
                m.propio("bc_color_solido", &v, &n, None, None, None)?
            }
        };
        lienzo = Some(if d.directo { m.mezclar(&l, &d.c, modo, d.op) } else { m.mover(&l, &d.c, d.m, modo, d.op) });
    }
    let l = lienzo?;
    match salida {
        Salida::Rgba => m.rgba8(&l),
        Salida::I420 => m.i420(&l),
    }
}
