//! Mesa de trabajo para Beethoven Craft: subir una capa, pasarle efectos, moverla, mezclarla y bajar el cuadro terminado.
//! Las piezas (núcleos, pozo de texturas, lectura) son las de EffectCraft.

use effectcraft_color::BlendMode;
use effectcraft_effects::Buf;
use effectcraft_geom::Mat3;
use effectcraft_raster::{Image, Sampling};

use crate::context::{Enc, GpuImage, Params};
use crate::effects::GBuf;
use crate::{FxStep, Gpu, ops};

/// Núcleos de `bc.wgsl`.
pub(crate) use crate::bc_lista::KERNELS;

/// Una capa en la GPU: imagen premultiplicada, con su desfase y escala (como `Buf`).
#[derive(Clone)]
pub struct Capa {
    img: GpuImage,
    pub offset: [f64; 2],
    pub scale: f64,
}

impl Capa {
    pub fn ancho(&self) -> u32 {
        self.img.width
    }
    pub fn alto(&self) -> u32 {
        self.img.height
    }
}

/// Parámetros de un núcleo propio: enteros (tres renglones) y decimales (once; el último lo llena la mesa).
#[derive(Clone, Copy, Default)]
pub struct Datos {
    pub u: [[u32; 4]; 4],
    pub f: [[f32; 4]; 12],
}

/// El trabajo de un cuadro.
pub struct Mesa<'g> {
    e: Enc<'g>,
    /// Texturas de los cuadros subidos: al terminar quedan para el siguiente.
    usadas: Vec<wgpu::Texture>,
}

impl Drop for Mesa<'_> {
    fn drop(&mut self) {
        self.e.submit();
        if let Ok(mut f) = self.e.g.frames.lock() {
            f.append(&mut self.usadas);
            // Unas cuantas nada más; las viejas (de otro tamaño) salen primero.
            let sobra = f.len().saturating_sub(12);
            f.drain(..sobra);
        }
    }
}

/// Cómo vienen los pixeles de un cuadro de video.
#[derive(Clone, Copy, PartialEq)]
pub enum Formato {
    /// RGBA de 8 bits sin premultiplicar.
    Rgba,
    /// Planos Y, U y V a 4:2:0, BT.709 de rango limitado.
    I420,
}

impl Gpu {
    /// `None` si la tarjeta falló: se sigue en el procesador.
    pub fn mesa(&self) -> Option<Mesa<'_>> {
        self.ctx.check_health().ok()?;
        self.ctx.can_readback().then(|| Mesa { e: Enc::lent(&self.ctx), usadas: vec![] })
    }
    pub fn nombre(&self) -> &str {
        &self.ctx.name
    }
    /// Se acabó un cuadro: el pozo de texturas suelta lo que ya nadie usa.
    pub fn fin_de_cuadro(&self) {
        self.ctx.pool_frame_end();
    }
    pub fn soporta(id: &str) -> bool {
        crate::effects::supports(id)
    }
}

impl Mesa<'_> {
    pub fn subir(&mut self, b: &Buf) -> Option<Capa> {
        Some(Capa { img: self.e.g.upload_image(&b.img)?, offset: b.offset, scale: b.scale })
    }

    /// Una imagen que se repite cuadro tras cuadro (títulos quietos): se sube una vez.
    pub fn subir_fijo(&mut self, img: &std::sync::Arc<Image>) -> Option<Capa> {
        Some(Capa { img: self.e.g.upload_picture(img)?, offset: [0.0; 2], scale: 1.0 })
    }

    /// Un cuadro de 8 bits, llevado a `dw` × `dh`.
    pub fn subir_video(&mut self, datos: &[u8], w: u32, h: u32, formato: Formato, (dw, dh): (u32, u32), scale: f64) -> Option<Capa> {
        let g = self.e.g;
        if !g.fits(w, h) || !g.fits(dw, dh) {
            return None;
        }
        let (tw, th, f, bytes) = match formato {
            Formato::Rgba => (w, h, wgpu::TextureFormat::Rgba8Unorm, w * 4),
            Formato::I420 => (w, h + h / 2, wgpu::TextureFormat::R8Unorm, w),
        };
        if datos.len() < (bytes * th) as usize || !g.fits(tw, th) {
            return None;
        }
        let libre = g.frames.lock().ok().and_then(|mut v| v.iter().position(|t| (t.width(), t.height(), t.format()) == (tw, th, f)).map(|i| v.swap_remove(i)));
        let t = libre.map_or_else(|| crate::context::init_resource(&g.device, "cuadro", || {
            g.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("cuadro"),
                size: wgpu::Extent3d { width: tw, height: th, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: f,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        }), Ok)
        .ok()?;
        self.usadas.push(t.clone());
        g.queue.write_texture(
            t.as_image_copy(),
            &datos[..(bytes * th) as usize],
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bytes), rows_per_image: Some(th) },
            wgpu::Extent3d { width: tw, height: th, depth_or_array_layers: 1 },
        );
        let src = GpuImage::new(t, tw, th);
        let mut p = Params::default();
        p.u[0] = [w, h, 0, 0];
        let out = self.e.scratch(dw, dh);
        self.e.pixels(if formato == Formato::Rgba { "bc_rgba" } else { "bc_i420" }, &p, &src, None, &out, None);
        (!self.e.take_missing()).then_some(Capa { img: out, offset: [0.0; 2], scale })
    }

    /// De regreso al procesador.
    pub fn bajar(&mut self, c: &Capa) -> Option<Buf> {
        let img: Image = self.e.download(&c.img)?;
        Some(Buf { img, offset: c.offset, scale: c.scale })
    }

    /// Efecto de EffectCraft. `None`: con esos ajustes va en el procesador.
    pub fn efecto(&mut self, c: &Capa, paso: &FxStep) -> Option<Capa> {
        self.e.take_missing();
        let b = GBuf { img: c.img.clone(), offset: c.offset, scale: c.scale };
        let b = crate::effects::apply(&mut self.e, paso.spec.id, &paso.ctx, b)?;
        self.e.mem_gate();
        (!self.e.take_missing()).then_some(Capa { img: b.img, offset: b.offset, scale: b.scale })
    }

    /// Núcleo propio sobre `c`, a `tam` (o del mismo tamaño).
    pub fn propio(&mut self, nucleo: &str, c: &Capa, d: &Datos, aux: Option<&Capa>, lista: Option<&[f32]>, tam: Option<(u32, u32)>) -> Option<Capa> {
        let (w, h) = tam.unwrap_or((c.img.width, c.img.height));
        if !self.e.g.fits(w, h) {
            return None;
        }
        let p = Params { u: d.u, f: d.f };
        let datos = lista.map(|v| self.e.data(if v.is_empty() { &[0.0; 4] } else { v }));
        let out = self.e.scratch(w, h);
        self.e.take_missing();
        self.e.pixels(nucleo, &p, &c.img, aux.map(|a| &a.img), &out, datos.as_ref());
        (!self.e.take_missing()).then_some(Capa { img: out, offset: c.offset, scale: c.scale })
    }

    /// Promedio de caja, `(rx, ry)` por pasada; repite la orilla.
    pub fn caja(&mut self, c: &Capa, pasadas: &[(usize, usize)]) -> Capa {
        let mut img = c.img.clone();
        for (rx, ry) in pasadas {
            img = crate::effects::box_passes(&mut self.e, &img, &[*rx], &[*ry], true);
        }
        Capa { img, offset: c.offset, scale: c.scale }
    }

    /// Desenfoque gaussiano; fuera de la imagen no hay nada.
    pub fn borroso(&mut self, c: &Capa, sigma: f64) -> Capa {
        if sigma <= 0.3 {
            return c.clone();
        }
        // Más allá de unas imágenes completas ya todo es parejo.
        let img = crate::effects::gaussian_blur(&mut self.e, &c.img, sigma.min(c.img.width as f64 * 2.0), sigma.min(c.img.height as f64 * 2.0), false);
        Capa { img, offset: c.offset, scale: c.scale }
    }

    /// A la mitad de tamaño.
    pub fn mitad(&mut self, c: &Capa) -> Capa {
        Capa { img: ops::half(&mut self.e, &c.img), offset: c.offset, scale: c.scale }
    }

    /// Dibuja `c` con la matriz `m` (afín, por renglones) sobre `lienzo`.
    pub fn mover(&mut self, lienzo: &Capa, c: &Capa, m: [f64; 6], modo: BlendMode, op: f32) -> Capa {
        let m = Mat3([[m[0], m[1], m[2]], [m[3], m[4], m[5]], [0.0, 0.0, 1.0]]);
        let img = ops::warp(&mut self.e, &lienzo.img, &c.img, &m, ops::WarpOptions { sampling: Sampling::Bilinear, mode: modo, opacity: op, seed: 0 }, None);
        Capa { img, offset: [0.0; 2], scale: lienzo.scale }
    }

    /// Encima, del mismo tamaño.
    pub fn mezclar(&mut self, lienzo: &Capa, c: &Capa, modo: BlendMode, op: f32) -> Capa {
        let img = ops::blend_full(&mut self.e, &lienzo.img, &c.img, modo, op, 0, None);
        Capa { img, offset: [0.0; 2], scale: lienzo.scale }
    }

    /// Un pedazo de `c`, o un lienzo más grande con ella.
    pub fn recorte(&mut self, c: &Capa, x: i64, y: i64, w: u32, h: u32) -> Capa {
        let img = self.e.crop(&c.img, x, y, w, h);
        Capa { img, offset: [0.0; 2], scale: c.scale }
    }

    pub fn vacia(&mut self, w: u32, h: u32, scale: f64) -> Option<Capa> {
        self.e.g.fits(w, h).then(|| Capa { img: self.e.image(w, h), offset: [0.0; 2], scale })
    }

    /// El cuadro terminado sobre negro, en RGBA de 8 bits.
    pub fn rgba8(&mut self, c: &Capa) -> Option<Vec<u8>> {
        let t = self.e.display_simple(&c.img);
        let mut v = self.e.read_texture(&t, c.img.width, c.img.height, 4)?;
        self.e.g.check_health().ok()?;
        v.iter_mut().skip(3).step_by(4).for_each(|a| *a = 255);
        Some(v)
    }

    /// El cuadro terminado sobre negro, en planos 4:2:0 BT.709. Ancho múltiplo de 8 y alto par.
    pub fn i420(&mut self, c: &Capa) -> Option<Vec<u8>> {
        let yp = self.e.g.yuv_pipe()?;
        let v = crate::yuv::convert(&mut self.e, yp, &c.img, [0.0; 3])?;
        self.e.g.check_health().ok()?;
        Some(v)
    }
}
