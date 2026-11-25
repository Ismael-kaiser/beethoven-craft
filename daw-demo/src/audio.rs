// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Salida de sonido.
use crate::idioma::t;
use crate::motor::Motor;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::Ordering::Relaxed;

pub struct Salida {
    _s: cpal::Stream,
    conf: cpal::SupportedStreamConfig,
}

/// Con qué frecuencia trabaja la salida.
pub fn frecuencia() -> Option<f32> {
    let d = cpal::default_host().default_output_device()?;
    Some(d.default_output_config().ok()?.sample_rate() as f32)
}

pub fn abrir(mut motor: Motor) -> Result<Salida, String> {
    let d = cpal::default_host().default_output_device().ok_or(t("No hay salida de audio"))?;
    let conf = d.default_output_config().map_err(|e| e.to_string())?;
    if conf.sample_format() != cpal::SampleFormat::F32 {
        return Err(t("La salida de audio no usa números de punto flotante").into());
    }
    let canales = conf.channels() as usize;
    let sr = conf.sample_rate() as f32;
    let mut c: cpal::StreamConfig = conf.clone().into();
    if let cpal::SupportedBufferSize::Range { min, max } = conf.buffer_size() {
        c.buffer_size = cpal::BufferSize::Fixed(256.clamp(*min, *max));
    }
    let mut b = vec![[0.0f32; 2]; 4096];
    let s = d
        .build_output_stream(
            c,
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                crate::receta::sin_denormales();
                let t = std::time::Instant::now();
                let cuadros = out.len() / canales;
                for trozo in out.chunks_mut(canales * 4096) {
                    let n = trozo.len() / canales;
                    motor.tocar(&mut b[..n]);
                    for (f, s) in trozo.chunks_exact_mut(canales).zip(&b) {
                        f.fill(0.0);
                        if canales == 1 {
                            f[0] = 0.5 * (s[0] + s[1]);
                        } else {
                            f[..2].copy_from_slice(s);
                        }
                    }
                }
                let uso = t.elapsed().as_secs_f32() / (cuadros.max(1) as f32 / sr) * 1000.0;
                let antes = motor.estado.carga.load(Relaxed) as f32;
                motor.estado.carga.store((antes * 0.95 + uso * 0.05) as u32, Relaxed);
            },
            |e| eprintln!("audio: {e}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    s.play().map_err(|e| e.to_string())?;
    Ok(Salida { _s: s, conf })
}

/// Grabación del micrófono. Al soltarla se cierra la entrada.
pub struct Entrada {
    _s: cpal::Stream,
    llega: crossbeam_channel::Receiver<Vec<f32>>,
    pub sr: u32,
    canales: usize,
    datos: Vec<[f32; 2]>,
}

impl Entrada {
    pub fn abrir() -> Result<Entrada, String> {
        let d = cpal::default_host().default_input_device().ok_or(t("No hay micrófono"))?;
        let conf = d.default_input_config().map_err(|e| e.to_string())?;
        let (manda, llega) = crossbeam_channel::unbounded();
        let s = d
            .build_input_stream(
                conf.clone().into(),
                move |x: &[f32], _: &cpal::InputCallbackInfo| _ = manda.send(x.to_vec()),
                |e| eprintln!("audio: {e}"),
                None,
            )
            .map_err(|e| e.to_string())?;
        s.play().map_err(|e| e.to_string())?;
        Ok(Entrada { _s: s, llega, sr: conf.sample_rate(), canales: conf.channels() as usize, datos: vec![] })
    }

    /// Junta lo que ha llegado.
    pub fn juntar(&mut self) -> &[[f32; 2]] {
        while let Ok(x) = self.llega.try_recv() {
            self.datos.extend(x.chunks_exact(self.canales.max(1)).map(|c| [c[0], c[if self.canales > 1 { 1 } else { 0 }]]));
        }
        &self.datos
    }
}

impl Salida {
    pub fn frecuencia(&self) -> f32 {
        self.conf.sample_rate() as f32
    }
}
