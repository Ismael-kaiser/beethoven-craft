// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! La playlist: patrones y audios acomodados en el tiempo.
use super::*;
use std::collections::{BTreeSet, HashMap};

pub struct Arreglo {
    zoom: f32,
    /// A dónde va el acercamiento: se llega suave.
    zoom_meta: f32,
    ancla: Option<f32>,
    scroll: Vec2,
    herramienta: Herramienta,
    iman: bool,
    elegidos: BTreeSet<usize>,
    copiados: Vec<Clip>,
    gesto: Option<(Gesto, Pos2)>,
    /// Los clips como estaban al empezar el gesto.
    base: Vec<Clip>,
    /// Lo que se ve mientras se arrastra, sin saltos: pulsos y pistas.
    flota: Vec2,
    libre: f32,
    /// Lo que falta para que lo soltado llegue a su lugar.
    caida: Vec2,
    /// Cuándo movió la vista la persona: la reproducción no se la quita.
    vio: f64,
    /// Dónde se pegó el imán.
    guia: Option<u32>,
    ondas: HashMap<String, Arc<Onda>>,
    /// Dónde quedó el área de los clips.
    area: Rect,
}

/// El dibujo de un audio: mínimo y máximo cada 256 muestras.
struct Onda {
    picos: Vec<[f32; 2]>,
    /// Segundos por pico.
    paso: f32,
    largo: f32,
}

impl Default for Arreglo {
    fn default() -> Self {
        Arreglo {
            zoom: 0.16,
            zoom_meta: 0.16,
            ancla: None,
            scroll: Vec2::ZERO,
            herramienta: Herramienta::Mover,
            iman: true,
            elegidos: BTreeSet::new(),
            copiados: vec![],
            gesto: None,
            base: vec![],
            flota: Vec2::ZERO,
            libre: 0.0,
            caida: Vec2::ZERO,
            vio: -10.0,
            guia: None,
            ondas: HashMap::new(),
            area: Rect::NOTHING,
        }
    }
}

impl Arreglo {
    /// La pista y el pulso de un punto de la ventana.
    pub fn lugar(&self, o: Pos2) -> (Option<usize>, u32) {
        if !self.area.contains(o) {
            return (None, 0);
        }
        let tick = ((o.x - self.area.left() + self.scroll.x) / self.zoom).max(0.0) as u32;
        let pista = ((o.y - self.area.top() + self.scroll.y) / ALTO).max(0.0) as usize;
        (Some(pista.min(PISTAS_ARREGLO - 1)), tick / PPQ * PPQ)
    }

    pub fn soltar(&mut self) {
        self.elegidos.clear();
        self.gesto = None;
    }

    pub fn elegir(&mut self, i: usize) {
        self.elegidos = BTreeSet::from([i]);
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Herramienta {
    Mover,
    Pincel,
    Tijera,
    Borrador,
}

#[derive(Clone, Copy, PartialEq)]
enum Gesto {
    Mover(usize),
    Alargar,
    Recortar,
    Entra(usize),
    Sale(usize),
    /// El tirador del centro: sube o baja todo el audio.
    Volumen(usize),
    /// Un punto de la curva de volumen.
    Punto(usize, usize),
    Borrar,
    Pintar,
    Marco,
    Cabezal,
    Mano,
}

const ALTO: f32 = 48.0;
const CABEZA: f32 = 118.0;
const REGLA: f32 = 30.0;

fn dibujo(h: Herramienta, p: &Painter, o: Pos2, c: Color32) {
    let st = Stroke::new(1.6, c);
    match h {
        Herramienta::Mover => {
            let v = [(-4.0, -7.0), (-4.0, 6.0), (-1.0, 3.0), (2.0, 8.0), (4.0, 7.0), (1.5, 2.0), (5.5, 2.0)];
            p.add(Shape::closed_line(v.map(|(x, y)| o + vec2(x, y)).to_vec(), st));
        }
        Herramienta::Pincel => {
            p.line_segment([o + vec2(6.0, -7.0), o + vec2(-2.0, 2.0)], Stroke::new(2.4, c));
            p.add(Shape::convex_polygon(vec![o + vec2(-3.0, 0.5), o + vec2(0.0, 3.5), o + vec2(-7.0, 7.0)], c, Stroke::NONE));
        }
        Herramienta::Tijera => {
            p.line_segment([o + vec2(-3.0, 3.0), o + vec2(6.0, -7.0)], st);
            p.line_segment([o + vec2(3.0, 3.0), o + vec2(-6.0, -7.0)], st);
            p.circle_stroke(o + vec2(-4.5, 5.0), 2.4, st);
            p.circle_stroke(o + vec2(4.5, 5.0), 2.4, st);
        }
        Herramienta::Borrador => {
            let v = [(-7.0, 2.0), (1.0, -6.0), (7.0, 0.0), (-1.0, 8.0)];
            p.add(Shape::closed_line(v.map(|(x, y)| o + vec2(x, y)).to_vec(), st));
            p.line_segment([o + vec2(-3.0, -2.0), o + vec2(3.0, 4.0)], st);
        }
    }
}

impl App {
    fn onda(&mut self, ruta: &str) -> Arc<Onda> {
        if let Some(o) = self.arreglo.ondas.get(ruta) {
            return o.clone();
        }
        let o = match self.bib.muestra(ruta, 0) {
            Some(m) if !m.zonas.is_empty() => {
                let z = &m.zonas[0];
                let x = &m.datos[z.ini..z.fin];
                let picos = x.chunks(256).map(|c| c.iter().fold([0f32, 0.0], |a, v| [a[0].min(*v), a[1].max(*v)])).collect();
                Onda { picos, paso: 256.0 / z.sr, largo: x.len() as f32 / z.sr }
            }
            _ => Onda { picos: vec![], paso: 1.0, largo: 0.0 },
        };
        let o = Arc::new(o);
        self.arreglo.ondas.insert(ruta.into(), o.clone());
        o
    }

    fn barra_arreglo(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            let lista = [
                (Herramienta::Mover, "Mover, recortar y elegir (⌘1)"),
                (Herramienta::Pincel, "Pincel: pinta el patrón elegido (⌘2)"),
                (Herramienta::Tijera, "Tijera: parte los clips (⌘3)"),
                (Herramienta::Borrador, "Borrador (⌘4)"),
            ];
            for (h, s) in lista {
                let el = self.arreglo.herramienta == h;
                if boton(ui, vec2(28.0, 26.0), el, if el { ACENTO } else { TEXTO }, t(s), |p, o, c| dibujo(h, p, o, c)).clicked() {
                    self.arreglo.herramienta = h;
                }
            }
            let i = self.arreglo.iman;
            if boton(
                ui,
                vec2(28.0, 26.0),
                i,
                if i { ACENTO } else { TEXTO },
                t("Imán: se pega a la rejilla, a otros clips y al cabezal"),
                |p, o, c| {
                    let st = Stroke::new(3.0, c);
                    let arco: Vec<Pos2> = (0..=10).map(|k| o + vec2(0.0, 0.5) + 5.0 * Vec2::angled(std::f32::consts::PI * k as f32 / 10.0)).collect();
                    p.add(Shape::line(arco, st));
                    p.vline(o.x + 5.0, o.y - 6.0..=o.y + 0.5, st);
                    p.vline(o.x - 5.0, o.y - 6.0..=o.y + 0.5, st);
                },
            )
            .clicked()
            {
                self.arreglo.iman = !i;
            }
            ui.add_space(4.0);
            for i in 0..self.p.patrones.len() {
                let x = &self.p.patrones[i];
                let (id, c, n) = (x.id, color(x.color), x.nombre.clone());
                let r = foco(ui, self.patron == id, &n, c).on_hover_text(t("Elígelo y píntalo en la canción · doble clic para editarlo"));
                if r.clicked() {
                    self.patron = id;
                }
                if r.double_clicked() {
                    self.mostrar(Vista::Ritmo);
                }
            }
            if ui.button(t("+ Pista")).on_hover_text(t("Pista nueva: elige el instrumento y se abre su piano roll")).clicked() {
                self.elegir = Some(lado::Elegir::pista(None));
            }
            if ui.button(t("+ Audio")).on_hover_text(t("Pone un audio en la canción; también puedes arrastrarlo o pegarlo")).clicked()
                && let Some(r) = rfd::FileDialog::new().add_filter("Audio", crate::decodificar::EXTENSIONES).pick_file()
            {
                let tick = self.tick().max(0.0) as u32 / PPQ * PPQ;
                if let Err(e) = self.poner_audio(&r, None, tick, None) {
                    self.avisar(e, true);
                }
            }
        });
    }

    /// El pulso más cercano al que se pega el imán, si hay uno cerca. Como en Movie: solo orillas, cabezal y, si se ven
    /// bien separadas, las rayas de la rejilla; así casi todo el tiempo el arrastre va libre.
    fn pegar_a(&self, tick: f32, zoom: f32, fuera: &BTreeSet<usize>) -> Option<u32> {
        let cerca = 6.0 / zoom;
        let mut v: Vec<f32> = vec![0.0, self.tick().round()];
        for (i, c) in self.arreglo.base.iter().enumerate() {
            if !fuera.contains(&i) {
                v.extend([c.tick as f32, (c.tick + c.dur) as f32]);
            }
        }
        if let Some(paso) = [PASO, PPQ, COMPAS, COMPAS * 4].into_iter().find(|p| *p as f32 * zoom > 44.0) {
            v.push((tick / paso as f32).round() * paso as f32);
        }
        v.into_iter().filter(|x| (x - tick).abs() <= cerca).min_by(|a, b| (a - tick).abs().total_cmp(&(b - tick).abs())).map(|x| x.max(0.0) as u32)
    }

    pub(super) fn ver_arreglo(&mut self, ui: &mut Ui) {
        self.barra_arreglo(ui);
        let (todo, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let regla = Rect::from_min_max(pos2(todo.left() + CABEZA, todo.top()), pos2(todo.right(), todo.top() + REGLA));
        let mapa = Rect::from_min_max(pos2(regla.left(), todo.bottom() - 16.0), todo.max);
        let r = Rect::from_min_max(pos2(todo.left() + CABEZA, regla.bottom()), pos2(todo.right(), mapa.top() - 3.0));
        let cab = Rect::from_min_max(pos2(todo.left(), r.top()), pos2(r.left(), r.bottom()));
        self.arreglo.area = r;
        let d = rueda(ui, todo);
        let (cmd, pin, dt) = ui.input(|i| (i.modifiers.command, i.zoom_delta(), i.stable_dt.min(0.05)));
        let hover = ui.input(|i| i.pointer.hover_pos()).filter(|o| todo.contains(*o) && ui.rect_contains_pointer(todo));
        let a = &mut self.arreglo;
        if let Some(o) = hover {
            let f = if cmd && d.y != 0.0 { (d.y / 220.0).exp() } else { pin };
            if f != 1.0 {
                a.zoom_meta = (a.zoom_meta * f).clamp(0.01, 2.0);
                a.ancla = Some(o.x);
            } else if !cmd {
                a.scroll -= d;
            }
        }
        // El acercamiento llega suave y deja quieto el punto bajo el ratón.
        if (a.zoom_meta / a.zoom - 1.0).abs() > 0.001 {
            let x0 = a.ancla.unwrap_or(r.left()) - r.left();
            let punto = (x0 + a.scroll.x) / a.zoom;
            a.zoom += (a.zoom_meta - a.zoom) * (1.0 - (-dt * 22.0).exp());
            a.scroll.x = punto * a.zoom - x0;
            ui.ctx().request_repaint();
        }
        let o_raton = ui.input(|i| i.pointer.interact_pos());
        // Arrastrar contra la orilla sigue de largo, más rápido mientras más afuera.
        if matches!(self.arreglo.gesto, Some((Gesto::Mover(_) | Gesto::Alargar | Gesto::Recortar | Gesto::Marco | Gesto::Pintar, _)))
            && let Some(o) = o_raton
        {
            let fuerza = |x: f32, lo: f32, hi: f32| (x - hi + 30.0).max(0.0) - (lo + 30.0 - x).max(0.0);
            let v = vec2(fuerza(o.x, r.left(), r.right()), fuerza(o.y, r.top(), r.bottom())) * dt * 14.0;
            if v != Vec2::ZERO {
                let antes = self.arreglo.scroll;
                self.arreglo.scroll = (antes + v).max(Vec2::ZERO);
                let corrio = self.arreglo.scroll - antes;
                if let Some((_, ini)) = &mut self.arreglo.gesto {
                    *ini -= corrio;
                }
                ui.ctx().request_repaint();
            }
        }
        if self.arreglo.caida != Vec2::ZERO {
            self.arreglo.caida *= (-dt * 26.0).exp();
            if (self.arreglo.caida.x * self.arreglo.zoom).abs() < 0.3 && (self.arreglo.caida.y * ALTO).abs() < 0.3 {
                self.arreglo.caida = Vec2::ZERO;
            }
            ui.ctx().request_repaint();
        }
        // Mientras suena, la vista acompaña al cabezal sin saltos; si la persona la movió, se le deja un rato.
        let ahora = ui.input(|i| i.time);
        if d != Vec2::ZERO || self.arreglo.gesto.is_some() || (self.arreglo.zoom_meta / self.arreglo.zoom - 1.0).abs() > 0.01 {
            self.arreglo.vio = ahora;
        }
        if self.tocando() && self.cancion && ahora - self.arreglo.vio > 2.0 {
            let x = self.tick() * self.arreglo.zoom - self.arreglo.scroll.x;
            let falta = if x > r.width() * 0.8 { x - r.width() * 0.8 } else { (x - 10.0).min(0.0) };
            self.arreglo.scroll.x += falta * (1.0 - (-dt * 5.0).exp());
        }
        let a = &mut self.arreglo;
        let total = (self.p.largo() + 16 * COMPAS) as f32;
        a.scroll.x = a.scroll.x.clamp(0.0, (total * a.zoom - r.width()).max(0.0));
        a.scroll.y = a.scroll.y.clamp(0.0, (PISTAS_ARREGLO as f32 * ALTO - r.height()).max(0.0));
        let (zoom, sc) = (a.zoom, a.scroll);
        let x_de = |k: f32| r.left() + k * zoom - sc.x;
        let y_de = |i: f32| r.top() + i * ALTO - sc.y;
        let tick_en = |x: f32| ((x - r.left() + sc.x) / zoom).max(0.0);
        let pista_en = |y: f32| (((y - r.top() + sc.y) / ALTO).max(0.0) as usize).min(PISTAS_ARREGLO - 1);
        let caja = |c: &Clip| {
            Rect::from_min_size(pos2(x_de(c.tick as f32), y_de(c.pista as f32)), vec2((c.dur as f32 * zoom).max(4.0), ALTO)).shrink2(vec2(0.5, 2.0))
        };
        // Lo que se está moviendo se dibuja donde va el ratón, sin brincos; al soltar cae en su lugar.
        let moviendo = matches!(self.arreglo.gesto, Some((Gesto::Mover(_), _)));
        let flota = self.arreglo.flota;
        let elegidos = self.arreglo.elegidos.clone();

        let p = ui.painter_at(r);
        p.rect_filled(r, 0.0, FONDO);
        for i in 0..PISTAS_ARREGLO {
            let f = Rect::from_min_size(pos2(r.left(), y_de(i as f32)), vec2(r.width(), ALTO));
            if f.intersects(r) {
                p.rect_filled(f, 0.0, if i % 2 == 0 { Color32::from_rgb(0x18, 0x19, 0x1d) } else { Color32::from_rgb(0x14, 0x15, 0x18) });
                p.hline(r.x_range(), f.bottom(), Stroke::new(1.0, Color32::BLACK));
            }
        }
        let pr = ui.painter_at(regla);
        // La regla como la carátula de un reloj: acero oscuro, minutería de riel e índices aplicados, bajo un cristal.
        let barra = Rect::from_min_max(pos2(todo.left(), regla.top()), regla.max);
        {
            let p = ui.painter_at(barra);
            degradado(&p, barra, 0, Color32::from_gray(0x36), Color32::from_gray(0x1c));
            p.hline(barra.x_range(), barra.top() + 0.5, Stroke::new(1.0, Color32::from_white_alpha(30)));
            p.hline(barra.x_range(), barra.bottom() - 0.5, Stroke::new(1.0, Color32::from_black_alpha(160)));
            // La ventanilla del compás, biselada y con su lupa; la esfera de segundos y sus engranes, que avanzan con el cabezal.
            let k = self.tick();
            let v = Rect::from_min_size(pos2(barra.left() + 5.0, barra.top() + 4.0), vec2(50.0, REGLA - 8.0));
            p.rect_filled(v.expand(1.5), 5, Color32::from_black_alpha(140));
            degradado(&p, v.expand(1.0), 5, Color32::from_gray(0x8a), Color32::from_gray(0x4a));
            hueco(&p, v.shrink(0.5), 4, Color32::from_gray(0x0c));
            let pulso = (k as u32 % COMPAS) / PPQ + 1;
            p.text(v.center(), Align2::CENTER_CENTER, format!("{}.{}", k as u32 / COMPAS + 1, pulso), letra(14.0), ACENTO);
            degradado_en(
                &p,
                Rect::from_min_max(v.min + vec2(1.5, 1.5), pos2(v.right() - 1.5, v.top() + v.height() * 0.48)),
                CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 },
                Color32::from_white_alpha(46),
                Color32::from_white_alpha(10),
            );
            let seg = k as f64 / PPQ as f64 * 60.0 / self.p.bpm as f64;
            let re = REGLA / 2.0 - 4.0;
            let o = pos2(barra.left() + CABEZA - re - 6.0, barra.center().y);
            let giro = (seg * 0.8) as f32;
            let g1 = o - vec2(re * 1.75, -1.0);
            engrane(&p, g1, re * 0.72, 16, -giro * 1.6, Color32::from_rgb(0x8c, 0x76, 0x35));
            p.circle_filled(g1, 2.2, Color32::from_rgb(0xc2, 0x2f, 0x4e));
            p.circle_filled(g1 - vec2(0.6, 0.8), 0.8, Color32::from_white_alpha(170));
            let g2 = o - vec2(re * 2.95, 1.5);
            engrane(&p, g2, re * 0.46, 11, giro * 2.5, Color32::from_rgb(0x84, 0x86, 0x8c));
            tornillo(&p, g2, 2.4, 0.7, true);
            esfera(&p, o, re, seg, if self.grabando.is_some() { ROJO } else { ACENTO });
        }
        let salto = [1, 2, 4, 8, 16, 32].into_iter().find(|s| (s * COMPAS) as f32 * zoom > 34.0).unwrap_or(64);
        if PPQ as f32 * zoom > 18.0 {
            let mut k = tick_en(r.left()) as u32 / PPQ * PPQ;
            while x_de(k as f32) < r.right() {
                p.vline(x_de(k as f32), r.y_range(), Stroke::new(1.0, RAYA.gamma_multiply(0.25)));
                k += PPQ;
            }
        }
        let mut c = tick_en(r.left()) as u32 / COMPAS;
        while x_de((c * COMPAS) as f32) < r.right() {
            let x = x_de((c * COMPAS) as f32);
            p.vline(x, r.y_range(), Stroke::new(1.0, RAYA.gamma_multiply(if c.is_multiple_of(4) { 1.0 } else { 0.5 })));
            if c.is_multiple_of(salto) {
                // Índice aplicado en el compás; entre compases, la minutería: una rayita por pulso.
                indice(&pr, x, regla.bottom() - 13.0, regla.bottom() - 4.0, 3.0, if c.is_multiple_of(4) { ACENTO } else { Color32::from_gray(0xd2) });
                let n = (c + 1).to_string();
                pr.text(pos2(x + 6.0, regla.top() + 5.0), Align2::LEFT_TOP, &n, letra(13.0), Color32::from_black_alpha(160));
                pr.text(pos2(x + 5.0, regla.top() + 4.0), Align2::LEFT_TOP, n, letra(13.0), TEXTO);
            }
            if PPQ as f32 * zoom > 7.0 {
                for j in 1..COMPAS / PPQ {
                    let xs = x_de((c * COMPAS + j * PPQ) as f32);
                    let largo = j == COMPAS / PPQ / 2;
                    pr.vline(
                        xs,
                        regla.bottom() - if largo { 11.0 } else { 9.0 }..=regla.bottom() - 4.5,
                        Stroke::new(if largo { 1.4 } else { 1.0 }, Color32::from_gray(0xa8)),
                    );
                }
            }
            c += 1;
        }
        // El riel de la minutería y el cristal de la carátula.
        for y in [regla.bottom() - 4.0, regla.bottom() - 9.5] {
            pr.hline(regla.x_range(), y, Stroke::new(0.7, Color32::from_white_alpha(38)));
        }
        degradado(
            &ui.painter_at(barra),
            Rect::from_min_max(barra.min, pos2(barra.right(), barra.top() + REGLA * 0.42)),
            0,
            Color32::from_white_alpha(22),
            Color32::from_white_alpha(4),
        );
        let puntero = hover.filter(|_| resp.hovered());
        // Los que se mueven van encima de los demás.
        let mut orden: Vec<usize> = (0..self.p.clips.len()).collect();
        if moviendo {
            orden.sort_by_key(|k| elegidos.contains(k));
        }
        for k in orden {
            let cl = self.p.clips[k].clone();
            let el = elegidos.contains(&k);
            let b = match self.arreglo.base.get(k).filter(|_| moviendo && el) {
                Some(x) => caja(x).translate(vec2(flota.x * zoom, flota.y * ALTO)),
                None if el => caja(&cl).translate(vec2(self.arreglo.caida.x * zoom, self.arreglo.caida.y * ALTO)),
                None => caja(&cl),
            };
            if moviendo && el {
                // Dónde va a caer.
                p.rect_stroke(caja(&cl), 5.0, Stroke::new(1.0, ACENTO.gamma_multiply(0.6)), StrokeKind::Inside);
            }
            if b.intersects(r) {
                self.dibujar_clip(&p, r, &cl, b, zoom, puntero.is_some_and(|o| b.contains(o)), el);
            }
        }
        if self.p.clips.is_empty() {
            p.text(r.center(), Align2::CENTER_CENTER, t("Elige un patrón y píntalo aquí con el pincel, o arrastra un audio"), letra(17.0), TENUE);
        }
        if let Some((e, tick)) = &mut self.grabando {
            let (n, sr) = (e.juntar().len(), e.sr);
            let dur = n as f32 / sr as f32 * self.p.bpm / 60.0 * PPQ as f32;
            encendido(&p, Rect::from_min_size(pos2(x_de(*tick as f32), r.top() + 4.0), vec2(dur * zoom, 14.0)), 3, ROJO);
        }
        if let Some(g) = self.arreglo.guia.filter(|_| self.arreglo.gesto.is_some()) {
            p.vline(x_de(g as f32), r.y_range(), Stroke::new(1.0, Color32::from_rgb(0x6a, 0xe0, 0xff)));
        }
        let toque = self.cabezas(ui, cab, puntero, &y_de);
        if let Some((i, Some(k))) = toque.filter(|_| resp.clicked()) {
            let x = &mut self.p.pistas[i];
            if k == 0 {
                x.mudo = !x.mudo;
            } else {
                x.solo = !x.solo;
            }
        }
        // El cabezal.
        if self.cancion {
            let x = x_de(self.tick());
            if x >= r.left() && x <= r.right() {
                let pt = ui.painter_at(regla.union(r));
                pt.vline(x, r.y_range(), Stroke::new(3.0, ACENTO.gamma_multiply(0.2)));
                pt.vline(x, r.y_range(), Stroke::new(1.5, ACENTO));
                let o = pos2(x, regla.bottom());
                let v = vec![o + vec2(-6.0, -13.0), o + vec2(6.0, -13.0), o + vec2(6.0, -6.0), o, o + vec2(-6.0, -6.0)];
                pt.add(Shape::convex_polygon(v, ACENTO, Stroke::new(1.0, Color32::BLACK)));
            }
        }
        // El mapa de toda la canción: arrastrarlo mueve la vista.
        let pm = ui.painter_at(mapa);
        hueco(&pm, mapa, 4, HUECO);
        let mx = |k: f32| mapa.left() + k / total * mapa.width();
        for cl in &self.p.clips {
            let col =
                if cl.audio.is_some() { Color32::from_rgb(0xd0, 0x7a, 0x7a) } else { self.p.patron(cl.patron).map_or(TENUE, |x| color(x.color)) };
            let y = mapa.top() + 2.0 + cl.pista as f32 / PISTAS_ARREGLO as f32 * (mapa.height() - 4.0);
            pm.rect_filled(
                Rect::from_min_max(pos2(mx(cl.tick as f32), y), pos2(mx((cl.tick + cl.dur) as f32).max(mx(cl.tick as f32) + 1.0), y + 1.5)),
                0.0,
                col,
            );
        }
        let ve = Rect::from_min_max(pos2(mx(sc.x / zoom), mapa.top()), pos2(mx((sc.x + r.width()) / zoom).min(mapa.right()), mapa.bottom()));
        pm.rect_filled(ve, 4.0, Color32::from_white_alpha(26));
        pm.rect_stroke(ve, 4.0, Stroke::new(1.0, Color32::from_white_alpha(70)), StrokeKind::Inside);

        self.teclas_arreglo(ui);
        let Some(o) = o_raton else { return };
        let sobre = self.p.clips.iter().rposition(|c| r.contains(o) && caja(c).contains(o));
        let h = self.arreglo.herramienta;
        let seg = 60.0 / self.p.bpm / PPQ as f32;
        let mut punto = None;
        let zona = sobre.map_or(0, |i| {
            let (b, c) = (caja(&self.p.clips[i]), &self.p.clips[i]);
            let ancho = 8.0_f32.min(b.width() * 0.3);
            let arriba = o.y < b.top() + 16.0 && c.audio.is_some() && b.width() > 40.0;
            let flecha = h == Herramienta::Mover;
            // En el cuerpo de un audio: los puntos de su curva y la curva misma.
            if let Some(a) = c.audio.as_ref().filter(|_| flecha && !arriba && b.width() > 40.0 && b.height() > 30.0) {
                let d = cuerpo(b);
                if let Some(j) = a.envolvente.iter().position(|q| (pos2(x_curva(b, a, q[0], zoom, seg), y_curva(d, q[1])) - o).length() < 6.0) {
                    punto = Some(j);
                    return 6;
                }
                let s = seg_curva(b, a, o.x, zoom, seg);
                if d.contains(o) && (y_curva(d, envolvente(&a.envolvente, s)) - o.y).abs() < 5.0 {
                    return 7;
                }
            }
            match o.x {
                x if arriba && flecha && (x - b.center().x).abs() < 10.0 && b.width() > 70.0 => 5,
                x if arriba && x < b.left() + 12.0 => 3,
                x if arriba && x > b.right() - 12.0 => 4,
                x if x < b.left() + ancho => 1,
                x if x > b.right() - ancho => 2,
                _ => 0,
            }
        });
        if resp.hovered() {
            self.guiar(ui, o, r, cab, sobre, zona);
            if h == Herramienta::Tijera
                && let Some(i) = sobre
            {
                let corte = self.corte_en(tick_en(o.x), zoom);
                p.vline(x_de(corte as f32), caja(&self.p.clips[i]).y_range(), Stroke::new(1.5, ROJO));
            }
        }
        // Menú del clic derecho.
        let dato = ui.id().with("menu");
        if ui.input(|i| i.pointer.secondary_pressed()) && resp.contains_pointer() {
            let que = if cab.contains(o) { Some((true, pista_en(o.y))) } else { sobre.map(|i| (false, i)) };
            if let Some((false, i)) = que
                && !self.arreglo.elegidos.contains(&i)
            {
                self.arreglo.elegir(i);
                self.clip = Some(i);
            }
            ui.data_mut(|d| d.insert_temp(dato, que));
        }
        let que: Option<(bool, usize)> = ui.data(|d| d.get_temp(dato)).flatten();
        if let Some((cabeza, i)) = que {
            let corte = tick_en(o.x) as u32;
            resp.context_menu(|ui| if cabeza { self.menu_pista(ui, i) } else { self.menu_clip(ui, i, corte) });
        }

        let (abajo, medio, suelto, mods) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.middle_down(), !i.pointer.any_down(), i.modifiers));
        let espacio = resp.contains_pointer();
        if medio && espacio && self.arreglo.gesto.is_none() {
            self.arreglo.gesto = Some((Gesto::Mano, o));
        }
        if abajo && espacio && mapa.contains(o) {
            self.arreglo.gesto = Some((Gesto::Mano, o));
            self.arreglo.scroll.x = (o.x - mapa.left()) / mapa.width() * total * zoom - r.width() / 2.0;
        } else if abajo && espacio && regla.contains(o) {
            self.cancion = true;
            self.sincronizar();
            self.arreglo.gesto = Some((Gesto::Cabezal, o));
        } else if abajo && espacio && r.contains(o) {
            self.arreglo.base = self.p.clips.clone();
            self.arreglo.flota = Vec2::ZERO;
            self.arreglo.libre = 0.0;
            self.arreglo.caida = Vec2::ZERO;
            let g = match (h, sobre) {
                (Herramienta::Borrador, _) => Gesto::Borrar,
                (Herramienta::Tijera, Some(i)) => {
                    let corte = self.corte_en(tick_en(o.x), zoom);
                    // Con varios elegidos, los parte a todos en el mismo lugar.
                    let quienes = if elegidos.contains(&i) { elegidos.clone() } else { BTreeSet::from([i]) };
                    self.partir(&quienes, corte);
                    Gesto::Mano
                }
                (Herramienta::Pincel, None) => Gesto::Pintar,
                (_, None) if mods.command || h == Herramienta::Mover => Gesto::Marco,
                (_, None) => Gesto::Marco,
                (_, Some(i)) => {
                    if mods.command {
                        // Suma o quita de lo elegido.
                        if !self.arreglo.elegidos.remove(&i) {
                            self.arreglo.elegidos.insert(i);
                        }
                    } else if !self.arreglo.elegidos.contains(&i) {
                        self.arreglo.elegir(i);
                    }
                    self.clip = Some(i);
                    match &self.p.clips[i].audio {
                        Some(a) => self.pista = a.mezcla,
                        None => self.patron = self.p.clips[i].patron,
                    }
                    let mut agarra = i;
                    if mods.shift && zona == 0 {
                        // Mayús: se lleva una copia.
                        let n = self.p.clips.len();
                        let viejos: Vec<usize> = self.arreglo.elegidos.iter().copied().collect();
                        agarra = n + viejos.iter().position(|k| *k == i).unwrap_or(0);
                        self.p.clips.extend(viejos.iter().map(|k| self.arreglo.base[*k].clone()));
                        self.arreglo.elegidos = (n..self.p.clips.len()).collect();
                        self.arreglo.base = self.p.clips.clone();
                    }
                    match zona {
                        1 => Gesto::Recortar,
                        2 => Gesto::Alargar,
                        3 => Gesto::Entra(i),
                        4 => Gesto::Sale(i),
                        5 => Gesto::Volumen(i),
                        6 => Gesto::Punto(i, punto.unwrap_or(0)),
                        7 => {
                            // Un punto nuevo donde se hizo clic, y se arrastra.
                            let b = caja(&self.p.clips[i]);
                            let a = self.p.clips[i].audio.as_mut().unwrap();
                            let s = seg_curva(b, a, o.x, zoom, seg);
                            let g = envolvente(&a.envolvente, s);
                            let j = a.envolvente.partition_point(|q| q[0] <= s);
                            a.envolvente.insert(j, [s, g]);
                            self.arreglo.base = self.p.clips.clone();
                            Gesto::Punto(i, j)
                        }
                        _ => Gesto::Mover(agarra),
                    }
                }
            };
            if g == Gesto::Marco && !mods.command {
                self.arreglo.elegidos.clear();
                self.clip = None;
            }
            self.arreglo.gesto = Some((g, o));
        }
        if resp.double_clicked()
            && let Some(i) = sobre
            && let Some(j) = punto
        {
            // Doble clic en un punto lo quita; en el tirador, el volumen vuelve a como estaba.
            if let Some(a) = self.p.clips[i].audio.as_mut().filter(|a| j < a.envolvente.len()) {
                a.envolvente.remove(j);
            }
            self.arreglo.gesto = None;
        } else if resp.double_clicked()
            && zona == 5
            && let Some(a) = sobre.and_then(|i| self.p.clips[i].audio.as_mut())
        {
            a.volumen = 1.0;
            self.arreglo.gesto = None;
        } else if resp.double_clicked()
            && h == Herramienta::Mover
            && let Some(i) = sobre
        {
            self.arreglo.gesto = None;
            if self.p.clips[i].audio.is_some() {
                self.clip = Some(i);
                self.mostrar(Vista::Editor);
            } else {
                self.abrir_patron();
            }
        }
        self.arrastrar(ui, o, r, &x_de, &tick_en, &pista_en, &caja, sobre, mods);
        if suelto
            && moviendo
            && let Some(k) = self.arreglo.elegidos.iter().next().copied()
        {
            // Lo que separa lo dibujado de donde quedó: se recorre en una décima.
            if let (Some(b), Some(c)) = (self.arreglo.base.get(k), self.p.clips.get(k)) {
                let quedo = vec2(c.tick as f32 - b.tick as f32, c.pista as f32 - b.pista as f32);
                self.arreglo.caida = self.arreglo.flota - quedo;
            }
        }
        if suelto {
            self.arreglo.gesto = None;
            self.arreglo.guia = None;
            self.arreglo.flota = Vec2::ZERO;
        }
    }

    /// Abre el patrón elegido donde convenga: en el piano roll si es melodía, o en Ritmo.
    fn abrir_patron(&mut self) {
        let Some(x) = self.p.patron(self.patron) else { return };
        let melodia = |v: &Vec<Nota>| v.iter().any(|n| n.tono != 60 || n.dur > PASO);
        if let Some(c) = x.notas.iter().find(|(c, v)| **c == self.canal && melodia(v)).or(x.notas.iter().find(|(_, v)| melodia(v))) {
            self.canal = *c.0;
            self.clip = None;
            self.notas.centrar();
            self.mostrar(Vista::Notas);
        } else if x.notas.is_empty() {
            self.notas.centrar();
            self.mostrar(Vista::Notas);
        } else {
            self.mostrar(Vista::Ritmo);
        }
    }

    fn corte_en(&self, tick: f32, zoom: f32) -> u32 {
        if self.arreglo.iman { self.pegar_a(tick, zoom, &BTreeSet::new()).unwrap_or(tick as u32) } else { tick as u32 }
    }

    /// Parte esos clips; lo elegido pasa a ser los pedazos de la derecha.
    pub fn partir(&mut self, quienes: &BTreeSet<usize>, en: u32) {
        let bpm = self.p.bpm;
        let n = self.p.clips.len();
        for i in quienes {
            if let Some(b) = self.p.clips.get_mut(*i).and_then(|c| c.cortar(en, bpm)) {
                self.p.clips.push(b);
            }
        }
        if self.p.clips.len() > n {
            self.arreglo.elegidos = (n..self.p.clips.len()).collect();
            self.clip = Some(n);
        }
    }

    pub fn borrar_clips(&mut self, quienes: &BTreeSet<usize>) {
        let mut i = 0;
        self.p.clips.retain(|_| {
            i += 1;
            !quienes.contains(&(i - 1))
        });
        self.arreglo.elegidos.clear();
        self.clip = None;
    }

    pub fn elegidos(&self) -> Vec<usize> {
        self.arreglo.elegidos.iter().copied().collect()
    }

    pub fn elegir_clips(&mut self, v: BTreeSet<usize>) {
        self.arreglo.elegidos = v;
    }

    /// Cuánto se aparta lo que se arrastra de donde estaba, en puntos.
    pub fn arrastre(&self) -> Vec2 {
        vec2(self.arreglo.flota.x * self.arreglo.zoom, self.arreglo.flota.y * ALTO)
    }

    pub fn duplicar_clips(&mut self) {
        let v: Vec<Clip> = self.arreglo.elegidos.iter().filter_map(|i| self.p.clips.get(*i).cloned()).collect();
        let a = v.iter().map(|c| c.tick).min().unwrap_or(0);
        let b = v.iter().map(|c| c.tick + c.dur).max().unwrap_or(0);
        self.poner_clips(v, b - a);
    }

    fn poner_clips(&mut self, v: Vec<Clip>, corre: u32) {
        let n = self.p.clips.len();
        self.p.clips.extend(v.into_iter().map(|c| Clip { tick: c.tick + corre, ..c }));
        self.arreglo.elegidos = (n..self.p.clips.len()).collect();
        self.clip = None;
    }

    fn arrastrar(
        &mut self,
        ui: &Ui,
        o: Pos2,
        r: Rect,
        x_de: &dyn Fn(f32) -> f32,
        tick_en: &dyn Fn(f32) -> f32,
        pista_en: &dyn Fn(f32) -> usize,
        caja: &dyn Fn(&Clip) -> Rect,
        sobre: Option<usize>,
        mods: Modifiers,
    ) {
        let Some((g, ini)) = self.arreglo.gesto else { return };
        let zoom = self.arreglo.zoom;
        let iman = self.arreglo.iman && !mods.alt;
        let el = self.arreglo.elegidos.clone();
        let base = self.arreglo.base.clone();
        let dt = (o.x - ini.x) / zoom;
        let seg = 60.0 / self.p.bpm / PPQ as f32;
        self.arreglo.guia = None;
        match g {
            Gesto::Mano => {
                let d = ui.input(|i| i.pointer.delta());
                if r.contains(ini) && ui.input(|i| i.pointer.middle_down()) {
                    self.arreglo.scroll -= d;
                } else if o.y > r.bottom() && ini.y > r.bottom() {
                    let total = (self.p.largo() + 16 * COMPAS) as f32 * zoom;
                    self.arreglo.scroll.x += d.x / r.width() * total;
                }
            }
            Gesto::Volumen(i) => {
                let Some(a0) = base.get(i).and_then(|c| c.audio.as_ref()) else { return };
                let fino = if mods.shift { 0.2 } else { 1.0 };
                // Un cuarto de dB por punto; se pega a 0 dB al pasar cerca.
                let mut db = (20.0 * a0.volumen.max(1e-4).log10() - (o.y - ini.y) * 0.25 * fino).clamp(-48.0, 12.0);
                if db.abs() < 0.4 && fino == 1.0 {
                    db = 0.0;
                }
                if let Some(a) = self.p.clips.get_mut(i).and_then(|c| c.audio.as_mut()) {
                    a.volumen = 10f32.powf(db / 20.0);
                }
            }
            Gesto::Punto(i, j) => {
                let Some(cl) = self.p.clips.get(i) else { return };
                let b = caja(cl);
                let d = cuerpo(b);
                let Some(a) = self.p.clips.get_mut(i).and_then(|c| c.audio.as_mut()) else { return };
                if j >= a.envolvente.len() {
                    return;
                }
                // Entre sus vecinos, para que no cambien de orden.
                let fin = a.inicio + cl_dur_seg(b, zoom, seg) * a.velocidad;
                let lo = if j > 0 { a.envolvente[j - 1][0] } else { a.inicio };
                let hi = a.envolvente.get(j + 1).map_or(fin, |q| q[0]);
                let s = seg_curva(b, a, o.x, zoom, seg).clamp(lo, hi.max(lo));
                let mut g = ((d.bottom() - o.y) / d.height() * 2.0).clamp(0.0, 2.0);
                if (g - 1.0).abs() < 0.04 {
                    g = 1.0;
                }
                a.envolvente[j] = [s, g];
            }
            Gesto::Cabezal => {
                let k = tick_en(o.x);
                let k = if iman { self.pegar_a(k, zoom, &BTreeSet::new()).map_or(k, |x| x as f32) } else { k };
                self.ir(k as f64);
            }
            Gesto::Mover(i) if i < base.len() => {
                let lim = el.iter().filter_map(|i| base.get(*i)).map(|c| c.tick as f32).fold(f32::MAX, f32::min);
                let libre = dt.max(-lim);
                let dy = (o.y - ini.y) / ALTO;
                let (lo, hi) = el.iter().filter_map(|i| base.get(*i)).fold((PISTAS_ARREGLO, 0), |m, c| (m.0.min(c.pista), m.1.max(c.pista)));
                let dy = dy.clamp(-(lo as f32), (PISTAS_ARREGLO - 1 - hi) as f32);
                // Menos de cuatro puntos no es arrastre: un clic no mueve nada.
                if (o - ini).length() < 4.0 && self.arreglo.flota == Vec2::ZERO {
                    return;
                }
                let mut final_ = libre;
                let mut pegado = libre;
                if iman {
                    // Se pega por el inicio o por el final del que se agarró.
                    let (a, b) = (base[i].tick as f32 + libre, (base[i].tick + base[i].dur) as f32 + libre);
                    let cand = [(a, self.pegar_a(a, zoom, &el)), (b, self.pegar_a(b, zoom, &el))];
                    let mejor = cand.into_iter().filter_map(|(x, s)| s.map(|s| (s, s as f32 - x))).min_by(|p, q| p.1.abs().total_cmp(&q.1.abs()));
                    if let Some((s, corre)) = mejor {
                        final_ = libre + corre;
                        self.arreglo.guia = Some(s);
                        pegado = final_;
                    }
                }
                // La pegada llega suave en vez de brincar; a lo alto sigue al ratón.
                let dt = ui.input(|i| i.stable_dt).min(0.05);
                let antes = self.arreglo.flota;
                let corrio = libre - self.arreglo.libre;
                self.arreglo.libre = libre;
                let x = antes.x + corrio;
                let x = x + (pegado - x) * (1.0 - (-dt * 30.0).exp());
                self.arreglo.flota = vec2(x, dy);
                if (x - pegado).abs() * zoom > 0.2 {
                    ui.ctx().request_repaint();
                }
                for k in &el {
                    if let (Some(b), Some(c)) = (base.get(*k), self.p.clips.get_mut(*k)) {
                        c.tick = (b.tick as f32 + final_).round().max(0.0) as u32;
                        c.pista = (b.pista as i32 + dy.round() as i32).clamp(0, PISTAS_ARREGLO as i32 - 1) as usize;
                    }
                }
            }
            Gesto::Alargar => {
                let Some(i) = self.clip.filter(|i| *i < base.len()) else { return };
                let fin = (base[i].tick + base[i].dur) as f32 + dt;
                let s = if iman { self.pegar_a(fin, zoom, &el) } else { None };
                self.arreglo.guia = s;
                let corre = s.map_or(fin, |x| x as f32) - (base[i].tick + base[i].dur) as f32;
                for k in &el {
                    let Some(b) = base.get(*k) else { continue };
                    let mut dur = (b.dur as f32 + corre).max(PASO as f32 / 2.0);
                    if let Some(a) = &b.audio {
                        // Un audio no se estira más allá de lo que trae.
                        dur = dur.min(((self.onda(&a.ruta).largo - a.inicio) / a.velocidad / seg).max(1.0));
                    }
                    self.p.clips[*k].dur = dur.round().max(1.0) as u32;
                }
            }
            Gesto::Recortar => {
                let Some(i) = self.clip.filter(|i| *i < base.len()) else { return };
                let x = base[i].tick as f32 + dt;
                let s = if iman { self.pegar_a(x, zoom, &el) } else { None };
                self.arreglo.guia = s;
                let corre = s.map_or(x, |x| x as f32) - base[i].tick as f32;
                for k in &el {
                    let Some(b) = base.get(*k) else { continue };
                    // Hacia atrás, hasta donde empieza el audio o el patrón.
                    let atras = b.audio.as_ref().map_or(b.desfase as f32, |a| a.inicio / a.velocidad / seg);
                    let corre = corre.clamp(-(b.tick as f32).min(atras), b.dur as f32 - PASO as f32 / 2.0).round();
                    let c = &mut self.p.clips[*k];
                    c.tick = (b.tick as f32 + corre) as u32;
                    c.dur = (b.dur as f32 - corre).max(1.0) as u32;
                    match (&mut c.audio, &b.audio) {
                        (Some(a), Some(z)) => a.inicio = (z.inicio + corre * seg * z.velocidad).max(0.0),
                        _ => c.desfase = (b.desfase as f32 + corre).max(0.0) as u32,
                    }
                }
            }
            Gesto::Entra(i) | Gesto::Sale(i) if i < self.p.clips.len() => {
                let b = caja(&self.p.clips[i]);
                let entra = matches!(g, Gesto::Entra(_));
                let px = if entra { o.x - b.left() } else { b.right() - o.x };
                let s = (px.clamp(0.0, b.width()) / zoom * seg).min(10.0);
                if let Some(a) = &mut self.p.clips[i].audio {
                    if entra {
                        a.entra = s;
                    } else {
                        a.sale = s;
                    }
                }
            }
            Gesto::Borrar => {
                if let Some(k) = sobre {
                    self.borrar_clips(&BTreeSet::from([k]));
                }
            }
            Gesto::Pintar => {
                let dur = self.p.patron(self.patron).map_or(COMPAS, |x| x.largo());
                let (pista, tick) = (pista_en(o.y), (tick_en(o.x) / dur as f32) as u32 * dur);
                let choca = self.p.clips.iter().any(|c| c.pista == pista && c.tick < tick + dur && c.tick + c.dur > tick);
                if r.contains(o) && !choca {
                    self.p.clips.push(Clip::de_patron(self.patron, pista, tick, dur));
                }
            }
            Gesto::Marco => {
                let m = Rect::from_two_pos(ini, o);
                if m.width() + m.height() > 5.0 {
                    let p = ui.painter_at(r);
                    p.rect_filled(m, 2.0, ACENTO.gamma_multiply(0.14));
                    p.rect_stroke(m, 2.0, Stroke::new(1.0, ACENTO), StrokeKind::Inside);
                    self.arreglo.elegidos = self.p.clips.iter().enumerate().filter(|(_, c)| caja(c).intersects(m)).map(|x| x.0).collect();
                }
            }
            _ => {}
        }
        let _ = x_de;
    }

    fn teclas_arreglo(&mut self, ui: &mut Ui) {
        if ui.ctx().text_edit_focused() || self.activa != Vista::Arreglo {
            return;
        }
        use egui::Key as K;
        let toco = |m: Modifiers, k: Key| ui.input_mut(|i| i.consume_key(m, k));
        // ⌘1 a ⌘4: los números solos tocan notas.
        for (k, h) in
            [(K::Num1, Herramienta::Mover), (K::Num2, Herramienta::Pincel), (K::Num3, Herramienta::Tijera), (K::Num4, Herramienta::Borrador)]
        {
            if toco(Modifiers::COMMAND, k) {
                self.arreglo.herramienta = h;
            }
        }
        if toco(Modifiers::COMMAND, K::A) {
            self.arreglo.elegidos = (0..self.p.clips.len()).collect();
        }
        if toco(Modifiers::COMMAND, K::E) {
            // Parte lo elegido en el cabezal; sin nada elegido, todo lo que cruza.
            let todos: BTreeSet<usize> = (0..self.p.clips.len()).collect();
            let q = if self.arreglo.elegidos.is_empty() { todos } else { self.arreglo.elegidos.clone() };
            self.partir(&q, self.tick().max(0.0) as u32);
        }
        // Lo copiado de aquí gana a los archivos: se vacía al copiar otra cosa.
        let (copia, corta, pega) = ui.input(|i| {
            let hay = |f: fn(&Event) -> bool| i.events.iter().any(f);
            (hay(|e| matches!(e, Event::Copy)), hay(|e| matches!(e, Event::Cut)), hay(|e| matches!(e, Event::Paste(_))))
        });
        let el = self.arreglo.elegidos.clone();
        if (copia || corta) && !el.is_empty() {
            self.arreglo.copiados = el.iter().filter_map(|i| self.p.clips.get(*i).cloned()).collect();
            ui.ctx().copy_text(MARCA.into());
            if corta {
                self.borrar_clips(&el);
                return;
            }
        }
        if pega && ui.input(|i| i.events.iter().any(|e| matches!(e, Event::Paste(s) if s == MARCA))) && !self.arreglo.copiados.is_empty() {
            let v = self.arreglo.copiados.clone();
            let a = v.iter().map(|c| c.tick).min().unwrap_or(0);
            let aqui = self.tick().max(0.0) as u32;
            let v = v.into_iter().map(|c| Clip { tick: c.tick - a, ..c }).collect();
            self.poner_clips(v, aqui);
        }
        if el.is_empty() {
            return;
        }
        if toco(Modifiers::NONE, K::Delete) || toco(Modifiers::NONE, K::Backspace) {
            return self.borrar_clips(&el);
        }
        if toco(Modifiers::COMMAND, K::D) || toco(Modifiers::COMMAND, K::B) {
            return self.duplicar_clips();
        }
        for (k, m, dx, dy) in [
            (K::ArrowLeft, Modifiers::NONE, -(PPQ as i64), 0i64),
            (K::ArrowRight, Modifiers::NONE, PPQ as i64, 0),
            (K::ArrowLeft, Modifiers::ALT, -1, 0),
            (K::ArrowRight, Modifiers::ALT, 1, 0),
            (K::ArrowUp, Modifiers::NONE, 0, -1),
            (K::ArrowDown, Modifiers::NONE, 0, 1),
        ] {
            if toco(m, k) {
                let cabe = el.iter().all(|i| {
                    let c = &self.p.clips[*i];
                    c.tick as i64 + dx >= 0 && (0..PISTAS_ARREGLO as i64).contains(&(c.pista as i64 + dy))
                });
                for i in el.iter().filter(|_| cabe) {
                    let c = &mut self.p.clips[*i];
                    c.tick = (c.tick as i64 + dx) as u32;
                    c.pista = (c.pista as i64 + dy) as usize;
                }
            }
        }
    }

    /// El cursor y la ayuda según lo que hay debajo.
    fn guiar(&mut self, ui: &Ui, o: Pos2, r: Rect, cab: Rect, sobre: Option<usize>, zona: u8) {
        let h = self.arreglo.herramienta;
        if r.contains(o) {
            ui.ctx().set_cursor_icon(match (h, sobre, zona) {
                (Herramienta::Tijera, ..) => CursorIcon::VerticalText,
                (Herramienta::Borrador, ..) => CursorIcon::NotAllowed,
                (_, Some(_), 1 | 2) => CursorIcon::ResizeHorizontal,
                (_, Some(_), 5) => CursorIcon::ResizeVertical,
                (_, Some(_), 6) => CursorIcon::Move,
                (_, Some(_), 7) => CursorIcon::Crosshair,
                (_, Some(_), 3 | 4) => CursorIcon::ResizeNeSw,
                (_, Some(_), _) if self.arreglo.gesto.is_some() => CursorIcon::Grabbing,
                (_, Some(_), _) => CursorIcon::Grab,
                (Herramienta::Pincel, ..) => CursorIcon::Crosshair,
                _ => CursorIcon::Default,
            });
        }
        self.ayuda(if cab.contains(o) {
            "M: silenciar · S: solo esta · clic derecho: nombre e instrumento"
        } else {
            match (h, sobre) {
                (Herramienta::Tijera, _) => "Clic: partir el clip ahí · ⌘/Ctrl+E: partir en el cabezal",
                (Herramienta::Borrador, _) => "Clic o arrastra: borrar",
                (_, Some(_)) => "Arrastra: mover · Mayús: copiar · bordes: recortar · esquinas de un audio: fundidos · tirador de arriba: volumen · línea blanca: clic pone un punto, doble clic lo quita · doble clic: abrir",
                (Herramienta::Pincel, _) => "Arrastra: pintar el patrón elegido",
                _ => "Arrastra: elegir varios · ⌘/Ctrl y rueda: acercar · rueda en medio: mover la vista · ⌘/Ctrl+C, V, D, E: copiar, pegar, duplicar, partir",
            }
        });
    }

    /// Las cabezas de las pistas. Dice sobre cuál está el ratón y en qué botón.
    fn cabezas(&mut self, ui: &Ui, cab: Rect, puntero: Option<Pos2>, y_de: &dyn Fn(f32) -> f32) -> Option<(usize, Option<usize>)> {
        let pc = ui.painter_at(cab);
        pc.rect_filled(cab, 0.0, FONDO);
        let mut toque = None;
        for i in 0..PISTAS_ARREGLO {
            let f = Rect::from_min_size(pos2(cab.left(), y_de(i as f32)), vec2(CABEZA - 3.0, ALTO)).shrink2(vec2(0.0, 1.0));
            if !f.intersects(cab) {
                continue;
            }
            let x = &self.p.pistas[i];
            let nombre = if x.nombre.is_empty() { tf("Pista {}", &[&(i + 1)]) } else { x.nombre.clone() };
            let suena = self.p.pista_suena(i);
            cabecera(&pc, f, &(i + 1).to_string(), suena, x.color.map(color));
            rotulo(&pc, Rect::from_min_size(f.min + vec2(44.0, 4.0), vec2(f.width() - 48.0, 18.0)), &nombre, 13.5, TEXTO);
            for (k, (s, si, c)) in [("M", x.mudo, ROJO), ("S", x.solo, ACENTO)].into_iter().enumerate() {
                let q = Rect::from_min_size(f.min + vec2(44.0 + 26.0 * k as f32, 24.0), vec2(22.0, 17.0));
                let hover = puntero.is_some_and(|o| q.contains(o) && cab.contains(o));
                if si {
                    encendido(&pc, q, 4, c);
                } else {
                    relieve(&pc, q, 4, CAJA, hover, false);
                }
                pc.text(q.center(), Align2::CENTER_CENTER, s, letra(12.0), if si { FONDO } else { TEXTO });
                if hover {
                    toque = Some((i, Some(k)));
                }
            }
            if toque.is_none_or(|t| t.0 != i) && puntero.is_some_and(|o| f.contains(o) && cab.contains(o)) {
                toque = Some((i, None));
            }
        }
        toque
    }

    fn menu_pista(&mut self, ui: &mut Ui, i: usize) {
        ui.set_min_width(190.0);
        ui.add(TextEdit::singleline(&mut self.p.pistas[i].nombre).hint_text(t("Nombre de la pista")));
        let x = &mut self.p.pistas[i];
        let mut c = x.color.unwrap_or([0x60; 3]);
        if mezcla::colores(ui, &mut c) {
            x.color = Some(c);
        }
        if x.color.is_some() && ui.button(t("Sin color")).clicked() {
            x.color = None;
        }
        if ui.button(t("Instrumento nuevo en esta pista…")).clicked() {
            self.elegir = Some(lado::Elegir::pista(Some(i)));
            ui.close();
        }
        if ui.button(t("Elegir todo lo de la pista")).clicked() {
            self.arreglo.elegidos = self.p.clips.iter().enumerate().filter(|(_, c)| c.pista == i).map(|x| x.0).collect();
            ui.close();
        }
    }

    fn menu_clip(&mut self, ui: &mut Ui, i: usize, _aqui: u32) {
        if i >= self.p.clips.len() {
            return;
        }
        ui.set_min_width(200.0);
        if let Some(c) = &mut self.p.clips[i].audio {
            ui.add(Slider::new(&mut c.volumen, 0.0..=2.0).text(t("Volumen")));
            ComboBox::from_id_salt("mezcla").selected_text(self.p.mezcla[c.mezcla].nombre.clone()).show_ui(ui, |ui| {
                for (k, m) in self.p.mezcla.iter().enumerate() {
                    ui.selectable_value(&mut c.mezcla, k, &m.nombre);
                }
            });
            if ui.button(t("Editar audio")).clicked() {
                self.clip = Some(i);
                self.mostrar(Vista::Editor);
                ui.close();
            }
            if ui.button(t("Ajustes del audio")).clicked() {
                self.clip = Some(i);
                self.mostrar(Vista::Canal);
                ui.close();
            }
        } else if ui.button(t("Abrir en Notas")).clicked() {
            self.patron = self.p.clips[i].patron;
            self.abrir_patron();
            ui.close();
        }
        ui.separator();
        let el = self.arreglo.elegidos.clone();
        let corte = self.tick().max(0.0) as u32;
        let cruza = el.iter().filter_map(|k| self.p.clips.get(*k)).any(|c| corte > c.tick && corte < c.tick + c.dur);
        if ui.add_enabled(cruza, Button::new(t("Cortar en el cabezal"))).clicked() {
            self.partir(&el, corte);
            ui.close();
        }
        if ui.button(t("Duplicar")).clicked() {
            self.duplicar_clips();
            ui.close();
        }
        if self.p.clips[i].audio.is_none() && ui.button(t("Hacer único")).on_hover_text(t("Le da su propia copia del patrón")).clicked() {
            if let Some(x) = self.p.patron(self.p.clips[i].patron).cloned() {
                let id = self.p.agregar_patron();
                let n = self.p.patrones.last_mut().unwrap();
                *n = Patron { id, nombre: tf("{} (copia)", &[&x.nombre]), color: n.color, ..x };
                self.p.clips[i].patron = id;
                self.patron = id;
            }
            ui.close();
        }
        if ui.button(t("Borrar")).clicked() {
            self.borrar_clips(&el);
            ui.close();
        }
    }

    fn dibujar_clip(&mut self, p: &Painter, r: Rect, cl: &Clip, b: Rect, zoom: f32, hover: bool, elegido: bool) {
        let apagado = !self.p.pista_suena(cl.pista);
        let (nombre, col) = match &cl.audio {
            Some(a) => (a.nombre.clone(), self.p.pistas[cl.pista].color.map_or(Color32::from_rgb(0xd0, 0x7a, 0x7a), color)),
            None => match self.p.patron(cl.patron) {
                Some(x) => (x.nombre.clone(), color(x.color)),
                None => return,
            },
        };
        let col = if apagado { mezcla(col, Color32::from_gray(0x50), 0.75) } else { col };
        p.rect_filled(b.translate(vec2(0.0, 2.0)), 6.0, Color32::from_black_alpha(70));
        let fondo = mezcla(col, Color32::from_gray(0x14), 0.62);
        degradado(p, b, 6, mezcla(fondo, Color32::WHITE, 0.1), mezcla(fondo, Color32::BLACK, 0.28));
        let franja = Rect::from_min_size(b.min, vec2(b.width(), 16.0));
        let cinta = mezcla(col, Color32::from_gray(0x30), 0.3);
        p.rect_filled(franja, CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }, Color32::from_rgba_unmultiplied(cinta.r(), cinta.g(), cinta.b(), 215));
        let pc = p.with_clip_rect(b.intersect(r));
        let dentro = Rect::from_min_max(pos2(b.left(), b.top() + 17.0), b.max).shrink(2.0);
        let luz = mezcla(col, Color32::WHITE, 0.55);
        if let Some(a) = &cl.audio {
            let o = self.onda(&a.ruta);
            let seg = 60.0 / self.p.bpm / PPQ as f32 / zoom * a.velocidad;
            for x in b.left().max(r.left()) as i32..b.right().min(r.right()) as i32 {
                let s0 = (x as f32 - b.left()) * seg + a.inicio;
                let (i, j) = ((s0.max(0.0) / o.paso) as usize, ((s0 + seg).max(0.0) / o.paso) as usize + 1);
                let Some(v) = o.picos.get(i..j.min(o.picos.len())).filter(|v| !v.is_empty()) else { continue };
                let m = v.iter().fold([0f32, 0.0], |m, q| [m[0].min(q[0]), m[1].max(q[1])]);
                let h = dentro.height() / 2.0 * a.volumen.min(1.2);
                let y = dentro.center().y;
                pc.vline(x as f32 + 0.5, (y - m[1] * h - 0.5)..=(y - m[0] * h + 0.5), Stroke::new(1.0, luz));
            }
            // Los fundidos, en sombra.
            let px = self.p.bpm / 60.0 * PPQ as f32 * zoom;
            let (e, f) = ((a.entra * px).min(b.width()), (a.sale * px).min(b.width()));
            for (w, x0, x1) in [(e, b.left(), b.left() + e), (f, b.right(), b.right() - f)] {
                if w > 1.0 {
                    let sombra = vec![pos2(x0, dentro.bottom()), pos2(x0, dentro.top()), pos2(x1, dentro.top())];
                    pc.add(Shape::convex_polygon(sombra, Color32::from_black_alpha(120), Stroke::NONE));
                    pc.line_segment([pos2(x0, dentro.bottom()), pos2(x1, dentro.top())], Stroke::new(1.5, luz));
                }
            }
        } else if let Some(pat) = self.p.patron(cl.patron).filter(|_| zoom > 0.03) {
            let (lo, hi) = pat.notas.values().flatten().fold((127, 0), |m, n| (m.0.min(n.tono), m.1.max(n.tono)));
            let alto = (hi as f32 - lo as f32).max(5.0) + 1.0;
            for (_, t, n, d) in pat.notas_en(cl.desfase, cl.dur) {
                let x = b.left() + t as f32 * zoom;
                let y = dentro.bottom() - (n.tono - lo + 1) as f32 / alto * (dentro.height() - 2.0);
                pc.rect_filled(Rect::from_min_size(pos2(x, y), vec2((d as f32 * zoom - 0.5).max(1.5), 2.5)), 0.0, luz);
            }
            let l = pat.largo();
            for k in (l - cl.desfase % l..cl.dur).step_by(l as usize) {
                pc.vline(b.left() + k as f32 * zoom, b.y_range(), Stroke::new(1.0, Color32::from_black_alpha(120)));
            }
        }
        cristal(p, b, col, hover);
        if let Some(a) = cl.audio.as_ref().filter(|_| b.width() > 40.0 && b.height() > 30.0) {
            self.curva_y_tirador(&pc, b, a, zoom, hover, elegido);
        }
        if elegido {
            p.rect_stroke(b, 5.0, Stroke::new(1.5, ACENTO), StrokeKind::Outside);
        }
        if hover && cl.audio.is_some() && b.width() > 40.0 {
            for q in [b.left_top() + vec2(6.0, 8.0), b.right_top() + vec2(-6.0, 8.0)] {
                p.circle_filled(q, 3.5, Color32::WHITE);
            }
        }
        let texto = Rect::from_min_size(b.min + vec2(if hover && cl.audio.is_some() { 14.0 } else { 7.0 }, 0.0), vec2(b.width() - 9.0, 16.0));
        rotulo(&pc, texto, &nombre, 13.0, Color32::WHITE);
    }
}

impl App {
    /// La curva de volumen encima de la onda, con sus puntos; y, al pasar, el tirador del centro con los dB.
    fn curva_y_tirador(&self, p: &Painter, b: Rect, a: &ClipAudio, zoom: f32, hover: bool, elegido: bool) {
        let seg = 60.0 / self.p.bpm / PPQ as f32;
        let d = cuerpo(b);
        let fuerte = hover || elegido;
        if fuerte || !a.envolvente.is_empty() {
            let (x0, x1) = (b.left().max(p.clip_rect().left()), b.right().min(p.clip_rect().right()));
            let mut v = vec![];
            let mut x = x0;
            while x <= x1 {
                v.push(pos2(x, y_curva(d, envolvente(&a.envolvente, seg_curva(b, a, x, zoom, seg)))));
                x += 3.0;
            }
            v.push(pos2(x1, y_curva(d, envolvente(&a.envolvente, seg_curva(b, a, x1, zoom, seg)))));
            let sombra: Vec<Pos2> = v.iter().map(|q| *q + vec2(0.0, 1.0)).collect();
            p.add(Shape::line(sombra, Stroke::new(1.0, Color32::from_black_alpha(140))));
            p.add(Shape::line(v, Stroke::new(if fuerte { 1.6 } else { 1.1 }, Color32::from_white_alpha(if fuerte { 230 } else { 150 }))));
            for q in &a.envolvente {
                let o = pos2(x_curva(b, a, q[0], zoom, seg), y_curva(d, q[1]));
                p.circle_filled(o, 4.0, Color32::WHITE);
                p.circle_stroke(o, 4.0, Stroke::new(1.0, Color32::from_black_alpha(170)));
            }
        }
        if hover && b.width() > 70.0 {
            // El tirador: se jala hacia arriba o abajo.
            let o = pos2(b.center().x.clamp(p.clip_rect().left() + 12.0, p.clip_rect().right() - 12.0), b.top() + 8.0);
            p.circle_filled(o + vec2(0.0, 1.0), 6.0, Color32::from_black_alpha(140));
            p.circle_filled(o, 6.0, Color32::from_gray(0xe8));
            p.circle_filled(o, 2.0, ACENTO);
            let db = 20.0 * a.volumen.max(1e-4).log10();
            let txt = format!("{db:+.1} dB");
            let g = p.layout_no_wrap(txt, letra(12.0), Color32::WHITE);
            let caja = Rect::from_min_size(pos2(o.x + 10.0, o.y - g.size().y / 2.0 - 1.0), g.size() + vec2(8.0, 2.0));
            p.rect_filled(caja, 4.0, Color32::from_black_alpha(190));
            p.galley(caja.min + vec2(4.0, 1.0), g, Color32::WHITE);
        }
    }
}

/// Dónde va la onda de un audio: debajo de su nombre.
fn cuerpo(b: Rect) -> Rect {
    Rect::from_min_max(pos2(b.left(), b.top() + 17.0), b.max).shrink(2.0)
}

/// Segundo del archivo bajo `x` en el clip `b`.
fn seg_curva(b: Rect, a: &ClipAudio, x: f32, zoom: f32, seg: f32) -> f32 {
    a.inicio + (x - b.left()) / zoom * seg * a.velocidad
}

fn x_curva(b: Rect, a: &ClipAudio, s: f32, zoom: f32, seg: f32) -> f32 {
    b.left() + (s - a.inicio) / a.velocidad.max(0.01) / seg * zoom
}

/// La ganancia 1 va a media altura y 2 hasta arriba.
fn y_curva(d: Rect, g: f32) -> f32 {
    d.bottom() - g.clamp(0.0, 2.0) / 2.0 * d.height()
}

fn cl_dur_seg(b: Rect, zoom: f32, seg: f32) -> f32 {
    b.width() / zoom * seg
}

/// Placa de la pista, como en Movie: metal con relieve, franja que se prende si suena, vidrio arriba y el número grabado
/// en un hueco con marco pulido y tornillos azulados.
fn cabecera(p: &Painter, r: Rect, clave: &str, suena: bool, col: Option<Color32>) {
    let caja = r.shrink2(vec2(1.0, 0.5));
    relieve(p, caja, 6, Color32::from_gray(0x34), false, false);
    let luz = match col {
        Some(c) if suena => c,
        Some(c) => mezcla(c, Color32::from_gray(0x40), 0.7),
        None if suena => ACENTO.gamma_multiply(0.85),
        None => Color32::from_gray(0x55),
    };
    let franja = Rect::from_min_max(caja.min + vec2(1.0, 1.0), pos2(caja.left() + 5.0, caja.bottom() - 1.0));
    degradado_en(p, franja, CornerRadius { nw: 5, sw: 5, ne: 0, se: 0 }, mezcla(luz, Color32::WHITE, 0.25), mezcla(luz, Color32::BLACK, 0.2));
    degradado_en(
        p,
        Rect::from_min_max(caja.min + vec2(5.0, 1.0), pos2(caja.right() - 1.0, caja.top() + caja.height() * 0.44)),
        CornerRadius { nw: 0, ne: 5, sw: 0, se: 0 },
        Color32::from_white_alpha(30),
        Color32::from_white_alpha(7),
    );
    let placa = Rect::from_min_max(caja.min + vec2(9.0, 4.0), pos2(caja.left() + 38.0, caja.bottom() - 4.0));
    degradado(p, placa.expand(0.8), 5, Color32::from_gray(0x74), Color32::from_gray(0x40));
    hueco(p, placa.shrink(0.3), 4, Color32::from_gray(0x1c));
    for (o, g) in [(placa.left_top() + vec2(4.5, 4.5), 0.5), (placa.right_bottom() - vec2(4.5, 4.5), 2.1)] {
        tornillo(p, o, 1.8, g, true);
    }
    let n = if clave.len() > 2 { 13.0 } else { 16.0 };
    p.text(placa.center() + vec2(0.0, 1.0), Align2::CENTER_CENTER, clave, letra(n), Color32::from_black_alpha(160));
    p.text(placa.center(), Align2::CENTER_CENTER, clave, letra(n), if suena { TEXTO } else { TENUE });
}

/// Encima del clip: un vidrio grueso, con su brillo, su luz de abajo y dos destellos en diagonal.
fn cristal(p: &Painter, b: Rect, col: Color32, hover: bool) {
    vidrio_en(p, b, 6, col, hover, 16.0);
    if b.width() > 90.0 {
        let (h, sesgo) = (b.height(), b.height() * 0.55);
        for (x, w, a) in [(0.10, 0.5, 13u8), (0.10 + 0.62 * h / b.width(), 0.16, 9)] {
            let x0 = b.left() + b.width().min(420.0) * x + sesgo;
            let w = h * w;
            let q = vec![
                pos2(x0, b.top() + 1.0),
                pos2(x0 + w, b.top() + 1.0),
                pos2(x0 + w - sesgo, b.bottom() - 1.0),
                pos2(x0 - sesgo, b.bottom() - 1.0),
            ];
            if q[1].x < b.right() - 4.0 {
                p.add(Shape::convex_polygon(q, Color32::from_white_alpha(a), Stroke::NONE));
            }
        }
    }
}

/// Lo que se deja en el portapapeles al copiar clips.
const MARCA: &str = "Beethoven DAW Demo: clips";
