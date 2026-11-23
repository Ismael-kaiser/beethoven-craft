//! Separa la voz del instrumental con un modelo MDX-Net (Ultimate Vocal Remover, «Voc FT»), todo en esta Mac.
use crate::medios::{self, Wav};
use realfft::{RealFftPlanner, num_complex::Complex32};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use tract_onnx::prelude::*;

/// Lo que pide el modelo: 44.1 kHz y estas medidas de espectrograma.
pub const SR44: u32 = 44_100;
const N: usize = 7680;
const SALTO: usize = 1024;
const F: usize = 3072;
const T: usize = 256;
/// Corrección de nivel que el modelo necesita (dato de sus autores).
const COMPENSAR: f32 = 1.021;
/// Cuadros útiles por tramo y los que se descartan de cada lado.
const TRAMO: usize = SALTO * (T - 1);
const ORILLA: usize = N / 2;
const UTIL: usize = TRAMO - 2 * ORILLA;

pub fn ruta_modelo() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    // Dentro del .app de Mac, o junto al programa en Windows y Linux.
    let mut lugares = vec![dir.join("../Resources/UVR-MDX-NET-Voc_FT.onnx"), dir.join("UVR-MDX-NET-Voc_FT.onnx")];
    lugares.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("modelos/UVR-MDX-NET-Voc_FT.onnx"));
    lugares.into_iter().find(|p| p.exists())
}

pub type Modelo = std::sync::Arc<TypedRunnableModel>;

pub fn cargar(ruta: &Path) -> Result<Modelo, String> {
    let f = || -> TractResult<Modelo> {
        tract_onnx::onnx()
            .model_for_path(ruta)?
            .with_input_fact(0, f32::fact([1, 4, F, T]).into())?
            .into_optimized()?
            .into_runnable()
    };
    f().map_err(|e| crate::idioma::tf("No pude cargar el modelo de separación: {}", &[&e]))
}

/// Ventana de Hann periódica.
fn ventana() -> Vec<f32> {
    (0..N).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / N as f32).cos()).collect()
}

/// Voz de un tramo de `TRAMO` cuadros.
fn tramo(m: &Modelo, pl: &mut RealFftPlanner<f32>, v: &[f32], x: &[[f32; 2]]) -> Result<Vec<[f32; 2]>, String> {
    let fft = pl.plan_fft_forward(N);
    let ifft = pl.plan_fft_inverse(N);
    let bins = N / 2 + 1;
    // Bordes reflejados, como torch.stft(center=True).
    let acol = |ch: usize| -> Vec<f32> {
        let n = x.len() as i64;
        (-(N as i64) / 2..n + N as i64 / 2)
            .map(|i| {
                let j = if i < 0 { -i } else if i >= n { 2 * (n - 1) - i } else { i };
                x[j as usize][ch]
            })
            .collect()
    };
    let mut esp = vec![0f32; 4 * F * T];
    let mut b = vec![0f32; N];
    let mut e = vec![Complex32::default(); bins];
    for ch in 0..2 {
        let s = acol(ch);
        for t in 0..T {
            for i in 0..N {
                b[i] = s[t * SALTO + i] * v[i];
            }
            fft.process(&mut b, &mut e).map_err(|e| e.to_string())?;
            for f in 0..F {
                esp[(2 * ch * F + f) * T + t] = e[f].re;
                esp[((2 * ch + 1) * F + f) * T + t] = e[f].im;
            }
        }
    }
    // Los 3 primeros renglones (retumbe) se anulan, como hace UVR.
    for c in 0..4 {
        esp[c * F * T..(c * F + 3) * T].fill(0.0);
    }
    let ent = Tensor::from_shape(&[1, 4, F, T], &esp).map_err(|e| e.to_string())?;
    let sal = m.run(tvec!(ent.into())).map_err(|e| crate::idioma::tf("Falló el modelo: {}", &[&e]))?;
    let y = sal[0].view();
    let y = y.as_slice::<f32>().map_err(|e| e.to_string())?;
    let mut out = vec![[0f32; 2]; x.len()];
    for ch in 0..2 {
        let mut suma = vec![0f32; x.len() + N];
        let mut peso = vec![0f32; x.len() + N];
        for t in 0..T {
            e.fill(Complex32::default());
            for f in 0..F {
                e[f] = Complex32::new(y[(2 * ch * F + f) * T + t], y[((2 * ch + 1) * F + f) * T + t]);
            }
            e[0].im = 0.0;
            ifft.process(&mut e, &mut b).map_err(|e| e.to_string())?;
            for i in 0..N {
                suma[t * SALTO + i] += b[i] * v[i] / N as f32;
                peso[t * SALTO + i] += v[i] * v[i];
            }
        }
        for (i, o) in out.iter_mut().enumerate() {
            o[ch] = suma[i + N / 2] / peso[i + N / 2].max(1e-8);
        }
    }
    Ok(out)
}

/// Separa un audio a 44.1 kHz en (voz, instrumental). `avance` va de 0 a 1000.
pub fn separar(
    m: &Modelo,
    x: &[[f32; 2]],
    avance: &AtomicU32,
    cancelar: &AtomicBool,
) -> Result<(Vec<[f32; 2]>, Vec<[f32; 2]>), String> {
    let v = ventana();
    let mut pl = RealFftPlanner::new();
    let mut voz = Vec::with_capacity(x.len() + UTIL);
    while voz.len() < x.len() {
        if cancelar.load(Relaxed) {
            return Err(crate::idioma::t("Separación cancelada").into());
        }
        let ini = voz.len() as i64 - ORILLA as i64;
        let trozo: Vec<[f32; 2]> = (ini..ini + TRAMO as i64)
            .map(|i| usize::try_from(i).ok().and_then(|i| x.get(i)).copied().unwrap_or_default())
            .collect();
        let y = tramo(m, &mut pl, &v, &trozo)?;
        voz.extend(y[ORILLA..ORILLA + UTIL].iter().map(|f| f.map(|s| s * COMPENSAR)));
        avance.store((voz.len() * 1000 / x.len()).min(1000) as u32, Relaxed);
    }
    voz.truncate(x.len());
    let ins = x.iter().zip(&voz).map(|(a, b)| [a[0] - b[0], a[1] - b[1]]).collect();
    Ok((voz, ins))
}

/// Separa un archivo y guarda los dos archivos en `carpeta`. Devuelve (voz, instrumental).
pub fn separar_archivo(
    ruta: &Path,
    carpeta: &Path,
    avance: &AtomicU32,
    cancelar: &AtomicBool,
) -> Result<(PathBuf, PathBuf), String> {
    let modelo = ruta_modelo().ok_or("Falta el modelo de separación")?;
    std::fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
    let base: String = ruta.file_stem().and_then(|s| s.to_str()).unwrap_or("audio").chars().take(60).collect();
    // El tamaño distingue dos archivos que se llamen igual.
    let tam = std::fs::metadata(ruta).map_err(|e| e.to_string())?.len();
    let nombre = |que: &str| carpeta.join(crate::idioma::tf("{} ({}) - {}.flac", &[&base, &tam, &que]));
    let (pv, pi) = (nombre("voz"), nombre("instrumental"));
    if pv.exists() && pi.exists() {
        return Ok((pv, pi));
    }
    let x = medios::decodificar_a(ruta, SR44)?;
    let m = cargar(&modelo)?;
    let (voz, ins) = separar(&m, &x, avance, cancelar)?;
    for (p, d) in [(&pv, voz), (&pi, ins)] {
        // FLAC ocupa menos de la mitad que WAV. Se renombra al final para no dejar archivos a medias.
        let (wav, tmp) = (p.with_extension("parcial.wav"), p.with_extension("parcial.flac"));
        let mut w = Wav::crear(&wav, 2, SR44).map_err(|e| e.to_string())?;
        w.escribir(d.as_flattened()).map_err(|e| e.to_string())?;
        w.cerrar().map_err(|e| e.to_string())?;
        let ok = medios::ffmpeg().arg("-y").arg("-i").arg(&wav).args(["-c:a", "flac"]).arg(&tmp).status();
        let _ = std::fs::remove_file(&wav);
        if !ok.is_ok_and(|s| s.success()) {
            return Err(crate::idioma::t("No pude guardar la separación").into());
        }
        std::fs::rename(&tmp, p).map_err(|e| e.to_string())?;
    }
    Ok((pv, pi))
}
