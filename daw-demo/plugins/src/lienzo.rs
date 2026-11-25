// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Lo que tienen en común las ventanas de los tres sistemas: egui dibujado por la tarjeta.
use beethoven_daw_demo::instancia::Instancia;
use beethoven_daw_demo::ui::externa::Externa;
use eframe::egui::{self, Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};
use egui_wgpu::wgpu;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::time::Instant;

struct Pintor {
    superficie: wgpu::Surface<'static>,
    conf: wgpu::SurfaceConfiguration,
    tarjeta: wgpu::Device,
    cola: wgpu::Queue,
    r: egui_wgpu::Renderer,
}

pub struct Lienzo {
    n: &'static Instancia,
    ctx: Context,
    externa: Externa,
    pintor: Option<Pintor>,
    /// Si la tarjeta no quiso, no se vuelve a intentar en cada cuadro.
    intentos: u32,
    eventos: Vec<Event>,
    ini: Instant,
    /// Píxeles por punto.
    pub escala: f32,
    /// En puntos.
    pub tam: (u32, u32),
    cada: Option<Box<dyn FnMut()>>,
    modos: Modifiers,
}

/// Cuánto mide la ventana de ese plugin, en puntos.
pub fn medida(n: &Instancia) -> (u32, u32) {
    let ctx = Context::default();
    let t = Externa::nueva(&ctx, n).tam(n);
    (t.x as u32, t.y as u32)
}

impl Lienzo {
    pub fn nuevo(n: &'static Instancia, escala: f32) -> Lienzo {
        let ctx = Context::default();
        let externa = Externa::nueva(&ctx, n);
        let t = externa.tam(n);
        Lienzo {
            n,
            ctx,
            externa,
            pintor: None,
            intentos: 0,
            eventos: vec![],
            ini: Instant::now(),
            escala,
            tam: (t.x as u32, t.y as u32),
            cada: None,
            modos: Modifiers::NONE,
        }
    }

    pub fn pixeles(&self) -> (u32, u32) {
        ((self.tam.0 as f32 * self.escala).round() as u32, (self.tam.1 as f32 * self.escala).round() as u32)
    }

    pub fn al_cuadro(&mut self, f: Box<dyn FnMut()>) {
        self.cada = Some(f);
    }

    pub fn modos(&mut self, m: Modifiers) {
        if m != self.modos {
            self.modos = m;
            self.eventos.push(Event::ModifiersChanged(m));
        }
    }

    pub fn raton(&mut self, o: Pos2) {
        self.eventos.push(Event::PointerMoved(o));
    }

    pub fn boton(&mut self, o: Pos2, b: PointerButton, si: bool) {
        self.eventos.push(Event::PointerMoved(o));
        self.eventos.push(Event::PointerButton { pos: o, button: b, pressed: si, modifiers: self.modos });
    }

    pub fn rueda(&mut self, x: f32, y: f32) {
        let (unit, phase) = (egui::MouseWheelUnit::Point, egui::TouchPhase::Move);
        self.eventos.push(Event::MouseWheel { unit, delta: vec2(x, y), phase, modifiers: self.modos });
    }

    pub fn se_fue(&mut self) {
        self.eventos.push(Event::PointerGone);
    }

    pub fn escribiendo(&self) -> bool {
        self.ctx.text_edit_focused()
    }

    pub fn tecla(&mut self, key: egui::Key) {
        for pressed in [true, false] {
            self.eventos.push(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: self.modos });
        }
    }

    pub fn texto(&mut self, s: &str) {
        let s: String = s.chars().filter(|c| !c.is_control()).collect();
        if !s.is_empty() {
            self.eventos.push(Event::Text(s));
        }
    }

    /// Suelta la tarjeta. Va antes de destruir la ventana.
    #[allow(dead_code)]
    pub fn soltar(&mut self) {
        self.pintor = None;
        self.intentos = 3;
    }

    /// Un cuadro. La ventana debe seguir viva.
    pub unsafe fn cuadro(&mut self, d: RawDisplayHandle, w: RawWindowHandle) {
        if let Some(f) = &mut self.cada {
            f();
        }
        let px = self.pixeles();
        if self.pintor.is_none() && self.intentos < 3 {
            self.intentos += 1;
            self.pintor = Pintor::nuevo(d, w, px);
        }
        let mut entra = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(self.tam.0 as f32, self.tam.1 as f32))),
            time: Some(self.ini.elapsed().as_secs_f64()),
            events: std::mem::take(&mut self.eventos),
            focused: true,
            ..Default::default()
        };
        entra.viewports.entry(egui::ViewportId::ROOT).or_default().native_pixels_per_point = Some(self.escala);
        let (n, externa) = (self.n, &mut self.externa);
        let o = self.ctx.run_ui(entra, |ui| externa.pintar(ui, n));
        let Some(p) = &mut self.pintor else { return };
        for (id, v) in &o.textures_delta.set {
            v.iter().for_each(|t| p.r.update_texture(&p.tarjeta, &p.cola, *id, t));
        }
        if (p.conf.width, p.conf.height) != px {
            (p.conf.width, p.conf.height) = (px.0.max(1), px.1.max(1));
            p.superficie.configure(&p.tarjeta, &p.conf);
        }
        let piezas = self.ctx.tessellate(o.shapes, o.pixels_per_point);
        let pantalla = egui_wgpu::ScreenDescriptor { size_in_pixels: [p.conf.width, p.conf.height], pixels_per_point: o.pixels_per_point };
        match p.superficie.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(c) | wgpu::CurrentSurfaceTexture::Suboptimal(c) => {
                let vista = c.texture.create_view(&Default::default());
                let mut enc = p.tarjeta.create_command_encoder(&Default::default());
                let extra = p.r.update_buffers(&p.tarjeta, &p.cola, &mut enc, &piezas, &pantalla);
                {
                    let mut paso = enc
                        .begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &vista,
                                resolve_target: None,
                                depth_slice: None,
                                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                            })],
                            ..Default::default()
                        })
                        .forget_lifetime();
                    p.r.render(&mut paso, &piezas, &pantalla);
                }
                p.cola.submit(extra.into_iter().chain([enc.finish()]));
                p.cola.present(c);
            }
            // Tapada o sin tiempo: será en el siguiente cuadro.
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {}
            _ => p.superficie.configure(&p.tarjeta, &p.conf),
        }
        for id in &o.textures_delta.free {
            p.r.free_texture(id);
        }
    }
}

impl Pintor {
    unsafe fn nuevo(d: RawDisplayHandle, w: RawWindowHandle, px: (u32, u32)) -> Option<Pintor> {
        let i = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let destino = wgpu::SurfaceTargetUnsafe::RawHandle { raw_display_handle: Some(d), raw_window_handle: w };
        let superficie = i.create_surface_unsafe(destino).ok()?;
        let a = espera(i.request_adapter(&wgpu::RequestAdapterOptions { compatible_surface: Some(&superficie), ..Default::default() })).ok()?;
        let (tarjeta, cola) = espera(a.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
        let mut conf = superficie.get_default_config(&a, px.0.max(1), px.1.max(1))?;
        // Los colores de egui ya van como se ven: sin la curva de sRGB.
        let formatos = superficie.get_capabilities(&a).formats;
        conf.format = formatos
            .iter()
            .copied()
            .find(|f| !f.is_srgb() && matches!(f, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm))
            .unwrap_or(conf.format);
        // Sin esperar a la pantalla: se dibuja en el hilo del otro programa.
        conf.present_mode = wgpu::PresentMode::AutoNoVsync;
        superficie.configure(&tarjeta, &conf);
        let r = egui_wgpu::Renderer::new(&tarjeta, conf.format, Default::default());
        Some(Pintor { superficie, conf, tarjeta, cola, r })
    }
}

/// Espera una respuesta de la tarjeta, que fuera de la web llega de inmediato.
fn espera<F: std::future::Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut c = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(x) = f.as_mut().poll(&mut c) {
            return x;
        }
        std::thread::yield_now();
    }
}
