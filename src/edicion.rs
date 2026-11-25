//! Operaciones de edición de la línea de tiempo, con imagen y sonido vinculados.
use crate::modelo::*;

const MIN: f64 = 0.05;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Orilla {
    Izq,
    Der,
}

/// Carril de un clip.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Carril {
    Video(usize),
    /// Id de la pista.
    Audio(u64),
}

/// Lo que tienen en común los clips de imagen y de sonido.
#[derive(Clone, Copy, Debug)]
pub struct Tramo {
    pub id: u64,
    pub carril: Carril,
    pub inicio: f64,
    pub dur: f64,
    pub desde: f64,
    pub total: f64,
    pub vel: f64,
    pub grupo: Option<u64>,
    /// Se puede estirar sin límite (títulos, imágenes, patrones).
    pub libre: bool,
    pub invertido: bool,
}

impl Tramo {
    pub fn fin(&self) -> f64 {
        self.inicio + self.dur
    }
    /// Material de sobra antes y después, en segundos de la línea de tiempo.
    fn sobra(&self) -> (f64, f64) {
        if self.libre {
            return (1e9, 1e9);
        }
        let (a, b) = (self.desde / self.vel, (self.total - self.desde) / self.vel - self.dur);
        if self.invertido { (b.max(0.0), a.max(0.0)) } else { (a.max(0.0), b.max(0.0)) }
    }
}

impl Proyecto {
    pub fn tramos(&self) -> Vec<Tramo> {
        let v = self.video.iter().map(|c| Tramo {
            id: c.id,
            carril: Carril::Video(c.capa),
            inicio: c.inicio,
            dur: c.dur,
            desde: c.desde,
            total: c.total,
            vel: c.mov.velocidad.max(0.01),
            grupo: c.grupo,
            libre: c.estirable(),
            invertido: false,
        });
        let a = self.pistas.iter().flat_map(|p| {
            let audio = p.clips.iter().map(|c| Tramo {
                id: c.id,
                carril: Carril::Audio(p.id),
                inicio: c.inicio,
                dur: c.dur,
                desde: c.desde,
                total: c.total,
                vel: c.velocidad.max(0.01),
                grupo: c.grupo,
                libre: false,
                invertido: c.invertido,
            });
            let pat = p.patrones.iter().map(|c| Tramo {
                id: c.id,
                carril: Carril::Audio(p.id),
                inicio: c.inicio,
                dur: c.dur,
                desde: 0.0,
                total: 1e9,
                vel: 1.0,
                grupo: None,
                libre: true,
                invertido: false,
            });
            audio.chain(pat).collect::<Vec<_>>()
        });
        v.chain(a).collect()
    }

    pub fn tramo_de(&self, id: u64) -> Option<Tramo> {
        self.tramos().into_iter().find(|t| t.id == id)
    }

    pub fn bloqueado(&self, c: Carril) -> bool {
        match c {
            Carril::Video(i) => self.capas.get(i).is_some_and(|c| c.bloqueada),
            Carril::Audio(id) => self.pistas.iter().any(|p| p.id == id && p.bloqueada),
        }
    }

    /// El clip y los vinculados con él, sin los de pistas bloqueadas.
    pub fn vinculados(&self, id: u64) -> Vec<Tramo> {
        let t = self.tramos();
        let g = t.iter().find(|x| x.id == id).and_then(|x| x.grupo);
        t.into_iter().filter(|x| (x.id == id || (g.is_some() && x.grupo == g)) && !self.bloqueado(x.carril)).collect()
    }

    /// Escribe inicio, dur y desde.
    fn poner_tramo(&mut self, t: &Tramo) {
        if let Some(c) = self.clip_video(t.id) {
            let d = t.inicio - c.inicio;
            let entro = t.desde - c.desde;
            (c.inicio, c.dur, c.desde) = (t.inicio, t.dur.max(MIN), t.desde);
            // Al recortar por la izquierda, las animaciones se quedan en su cuadro.
            if d.abs() > 1e-9 && entro.abs() > 1e-9 {
                let corre = entro / c.mov.velocidad.max(0.01);
                c.animables().for_each(|p| p.correr(-corre));
            }
            return;
        }
        for p in &mut self.pistas {
            if let Some(c) = p.clips.iter_mut().find(|c| c.id == t.id) {
                (c.inicio, c.dur, c.desde) = (t.inicio, t.dur.max(MIN), t.desde);
                c.fade_in = c.fade_in.min(c.dur);
                c.fade_out = c.fade_out.min(c.dur - c.fade_in);
                return;
            }
            if let Some(c) = p.patrones.iter_mut().find(|c| c.id == t.id) {
                (c.inicio, c.dur) = (t.inicio, t.dur.max(MIN));
                return;
            }
        }
    }

    /// Vincula clips (los que se pasan) en un grupo nuevo.
    pub fn vincular(&mut self, ids: &[u64]) {
        let g = self.id();
        self.poner_grupo(ids, Some(g));
    }

    pub fn desvincular(&mut self, id: u64) {
        let ids: Vec<u64> = self.vinculados(id).iter().map(|t| t.id).collect();
        self.poner_grupo(&ids, None);
    }

    fn poner_grupo(&mut self, ids: &[u64], g: Option<u64>) {
        self.video.iter_mut().filter(|c| ids.contains(&c.id)).for_each(|c| c.grupo = g);
        self.pistas.iter_mut().flat_map(|p| &mut p.clips).filter(|c| ids.contains(&c.id)).for_each(|c| c.grupo = g);
    }

    /// Mueve un clip y sus vinculados. Nadie queda antes del cero.
    pub fn mover_grupo(&mut self, id: u64, d: f64) {
        let v = self.vinculados(id);
        let d = v.iter().map(|t| -t.inicio).fold(d, f64::max);
        for mut t in v {
            t.inicio += d;
            self.poner_tramo(&t);
        }
    }

    /// Recorre `d` segundos todo lo que empieza en `t` o después (menos `menos`), en todas las pistas sin bloquear.
    pub fn empujar(&mut self, t: f64, d: f64, menos: &[u64]) {
        for mut x in self.tramos() {
            if x.inicio >= t - 1e-6 && !menos.contains(&x.id) && !self.bloqueado(x.carril) {
                x.inicio = (x.inicio + d).max(0.0);
                self.poner_tramo(&x);
            }
        }
        for m in &mut self.marcas {
            if m.t >= t - 1e-6 {
                m.t = (m.t + d).max(0.0);
            }
        }
        for a in self.pistas.iter_mut().filter(|p| !p.bloqueada).flat_map(|p| &mut p.auto) {
            a.puntos.iter_mut().filter(|q| q.0 >= t - 1e-6).for_each(|q| q.0 = (q.0 + d).max(0.0));
        }
    }

    /// Cuánto se puede cerrar un hueco que termina en `t` sin encimar nada.
    fn cierre(&self, t: f64, d: f64, menos: &[u64]) -> f64 {
        let tr = self.tramos();
        let mut max = d;
        for x in tr.iter().filter(|x| x.inicio >= t - 1e-6 && !self.bloqueado(x.carril)) {
            // Lo que no se mueve de su mismo carril es la pared.
            let pared = tr.iter().filter(|y| y.carril == x.carril && y.inicio < t - 1e-6 && !menos.contains(&y.id)).map(|y| y.fin()).fold(0.0, f64::max);
            max = max.min(x.inicio - pared);
        }
        max.max(0.0)
    }

    /// Borra el clip y sus vinculados y cierra el hueco.
    pub fn borrar_ondulado(&mut self, id: u64) {
        let v = self.vinculados(id);
        let (Some(a), Some(b)) = (v.iter().map(|t| t.inicio).reduce(f64::min), v.iter().map(|t| t.fin()).reduce(f64::max)) else { return };
        for t in &v {
            self.borrar_clip(t.id);
        }
        let d = self.cierre(b, b - a, &[]);
        self.empujar(b, -d, &[]);
    }

    pub fn borrar_grupo(&mut self, id: u64) {
        for t in self.vinculados(id) {
            self.borrar_clip(t.id);
        }
    }

    /// Recorta una orilla sin mover lo demás. Devuelve lo que sí se pudo.
    pub fn recortar(&mut self, id: u64, o: Orilla, d: f64) -> f64 {
        let v = self.vinculados(id);
        let d = v.iter().fold(d, |d, t| limitar(t, o, d));
        for mut t in v {
            aplicar(&mut t, o, d);
            self.poner_tramo(&t);
        }
        d
    }

    /// Recorte que arrastra todo lo que sigue, sin dejar hueco.
    pub fn recorte_ondulado(&mut self, id: u64, o: Orilla, d: f64) -> f64 {
        let v = self.vinculados(id);
        let Some(fin) = v.iter().map(|t| t.fin()).reduce(f64::max) else { return 0.0 };
        let ids: Vec<u64> = v.iter().map(|t| t.id).collect();
        let mut d = v.iter().fold(d, |d, t| limitar(t, o, d));
        // `cambio`: cuánto crece la duración.
        let cambio = if o == Orilla::Der { d } else { -d };
        if cambio < 0.0 {
            let c = -self.cierre(fin, -cambio, &ids);
            d = if o == Orilla::Der { c } else { -c };
        }
        let cambio = if o == Orilla::Der { d } else { -d };
        if cambio > 0.0 {
            self.empujar(fin, cambio, &ids);
        }
        for mut t in v {
            let ini = t.inicio;
            aplicar(&mut t, o, d);
            t.inicio = ini;
            self.poner_tramo(&t);
        }
        if cambio < 0.0 {
            self.empujar(fin, cambio, &ids);
        }
        d
    }

    /// Vecino pegado en esa orilla, en el mismo carril.
    pub fn vecino(&self, t: &Tramo, o: Orilla) -> Option<Tramo> {
        self.tramos().into_iter().find(|x| {
            x.id != t.id && x.carril == t.carril && match o {
                Orilla::Izq => (x.fin() - t.inicio).abs() < 0.02,
                Orilla::Der => (x.inicio - t.fin()).abs() < 0.02,
            }
        })
    }

    /// Mueve el corte entre dos clips pegados: uno crece lo que el otro se encoge.
    pub fn rodar(&mut self, id: u64, o: Orilla, d: f64) -> f64 {
        let pares: Vec<(Tramo, Tramo)> = self
            .vinculados(id)
            .into_iter()
            .filter_map(|t| {
                let v = self.vecino(&t, o)?;
                Some(if o == Orilla::Der { (t, v) } else { (v, t) })
            })
            .collect();
        let d = pares.iter().fold(d, |d, (a, b)| limitar(b, Orilla::Izq, limitar(a, Orilla::Der, d)));
        if pares.is_empty() {
            return 0.0;
        }
        for (mut a, mut b) in pares {
            aplicar(&mut a, Orilla::Der, d);
            aplicar(&mut b, Orilla::Izq, d);
            self.poner_tramo(&a);
            self.poner_tramo(&b);
        }
        d
    }

    /// Cambia qué parte del archivo se ve, sin mover el clip. `d` en segundos de línea de tiempo.
    pub fn deslizar(&mut self, id: u64, d: f64) {
        let v = self.vinculados(id);
        let d = v.iter().fold(d, |d, t| {
            let (antes, despues) = t.sobra();
            if t.libre { d } else { d.clamp(-antes, despues) }
        });
        for mut t in v.into_iter().filter(|t| !t.libre) {
            t.desde = (t.desde + if t.invertido { -d } else { d } * t.vel).max(0.0);
            self.poner_tramo(&t);
        }
    }

    /// Mueve el clip entre sus vecinos, que crecen y se encogen para no dejar huecos.
    pub fn correr(&mut self, id: u64, d: f64) -> f64 {
        let v = self.vinculados(id);
        let mut d = v.iter().map(|t| -t.inicio).fold(d, f64::max);
        let lados: Vec<(Tramo, Option<Tramo>, Option<Tramo>)> =
            v.into_iter().map(|t| (t, self.vecino(&t, Orilla::Izq), self.vecino(&t, Orilla::Der))).collect();
        for (_, a, b) in &lados {
            if let Some(a) = a {
                d = limitar(a, Orilla::Der, d);
            }
            if let Some(b) = b {
                d = limitar(b, Orilla::Izq, d);
            }
        }
        for (mut t, a, b) in lados {
            t.inicio += d;
            self.poner_tramo(&t);
            if let Some(mut a) = a {
                aplicar(&mut a, Orilla::Der, d);
                self.poner_tramo(&a);
            }
            if let Some(mut b) = b {
                aplicar(&mut b, Orilla::Izq, d);
                self.poner_tramo(&b);
            }
        }
        d
    }

    /// Cambia la velocidad para que el clip dure `dur`, mostrando el mismo tramo. Devuelve la velocidad.
    pub fn estirar(&mut self, id: u64, dur: f64) -> Option<f64> {
        let t = self.tramo_de(id).filter(|t| !t.libre)?;
        let v = (t.vel * t.dur / dur.max(MIN)).clamp(0.1, 10.0);
        self.velocidad(id, v);
        Some(v)
    }

    /// Pone la velocidad de un clip y sus vinculados.
    pub fn velocidad(&mut self, id: u64, v: f64) {
        for t in self.vinculados(id).into_iter().filter(|t| !t.libre) {
            let dur = (t.dur * t.vel / v).max(MIN);
            if let Some(c) = self.clip_video(t.id) {
                c.mov.velocidad = v;
                c.dur = dur;
            } else if let Some(c) = self.clip(t.id) {
                c.velocidad = v;
                c.dur = dur;
                c.fade_in = c.fade_in.min(dur);
                c.fade_out = c.fade_out.min(dur - c.fade_in);
            }
        }
    }

    /// Divide el clip y sus vinculados; los pedazos de la derecha quedan vinculados entre sí.
    pub fn dividir_grupo(&mut self, id: u64, t: f64) -> bool {
        let v = self.vinculados(id);
        let con_grupo = v.iter().any(|x| x.grupo.is_some());
        let mut nuevos = vec![];
        for x in v {
            let antes: Vec<u64> = self.tramos().iter().map(|y| y.id).collect();
            if self.dividir(x.id, t) {
                nuevos.extend(self.tramos().iter().map(|y| y.id).filter(|y| !antes.contains(y)));
            }
        }
        if con_grupo && !nuevos.is_empty() {
            self.vincular(&nuevos);
        }
        !nuevos.is_empty()
    }

    /// Corta todo lo que pase por `t`.
    pub fn dividir_todo(&mut self, t: f64) {
        let mut hechos: Vec<u64> = vec![];
        for x in self.tramos() {
            if t > x.inicio + 0.01 && t < x.fin() - 0.01 && !self.bloqueado(x.carril) && !hechos.contains(&x.id) {
                hechos.extend(self.vinculados(x.id).iter().map(|y| y.id));
                self.dividir_grupo(x.id, t);
            }
        }
    }

    /// Vacía [a, b] en un carril, recortando o partiendo lo que estorbe.
    pub fn vaciar(&mut self, carril: Carril, a: f64, b: f64, menos: &[u64]) {
        for x in self.tramos().into_iter().filter(|x| x.carril == carril && !menos.contains(&x.id)) {
            if x.fin() <= a + 1e-6 || x.inicio >= b - 1e-6 {
                continue;
            }
            if x.inicio < a - 0.01 && x.fin() > b + 0.01 {
                // Queda a los dos lados.
                let antes: Vec<u64> = self.tramos().iter().map(|y| y.id).collect();
                if self.dividir(x.id, a) {
                    if let Some(n) = self.tramos().into_iter().find(|y| !antes.contains(&y.id)) {
                        let mut n = n;
                        aplicar(&mut n, Orilla::Izq, b - a);
                        self.poner_tramo(&n);
                    }
                }
            } else if x.inicio < a - 0.01 {
                let mut t = x;
                aplicar(&mut t, Orilla::Der, a - x.fin());
                self.poner_tramo(&t);
            } else if x.fin() > b + 0.01 {
                let mut t = x;
                aplicar(&mut t, Orilla::Izq, b - x.inicio);
                self.poner_tramo(&t);
            } else {
                self.borrar_clip(x.id);
            }
        }
    }

    /// Levanta (deja hueco) o extrae (lo cierra) un tramo de todas las pistas.
    pub fn quitar_rango(&mut self, a: f64, b: f64, cerrar: bool) {
        if b - a < 0.01 {
            return;
        }
        let mut carriles: Vec<Carril> = (0..self.capas.len()).map(Carril::Video).collect();
        carriles.extend(self.pistas.iter().map(|p| Carril::Audio(p.id)));
        for c in carriles {
            if !self.bloqueado(c) {
                self.vaciar(c, a, b, &[]);
            }
        }
        if cerrar {
            let d = self.cierre(b, b - a, &[]);
            self.empujar(b, -d, &[]);
        }
    }

    /// Los puntos donde hay un corte, en orden.
    pub fn cortes(&self) -> Vec<f64> {
        let mut v: Vec<f64> = self.tramos().iter().flat_map(|t| [t.inicio, t.fin()]).chain(self.marcas.iter().map(|m| m.t)).collect();
        v.push(0.0);
        v.sort_by(f64::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 0.005);
        v
    }
}

/// Cuánto se puede mover una orilla.
fn limitar(t: &Tramo, o: Orilla, d: f64) -> f64 {
    let (antes, despues) = t.sobra();
    match o {
        Orilla::Der => d.clamp(MIN - t.dur, despues),
        Orilla::Izq => d.clamp(-(antes.min(t.inicio)), t.dur - MIN),
    }
}

fn aplicar(t: &mut Tramo, o: Orilla, d: f64) {
    match o {
        Orilla::Der => {
            t.dur += d;
            if t.invertido {
                t.desde -= d * t.vel;
            }
        }
        Orilla::Izq => {
            t.inicio += d;
            t.dur -= d;
            if !t.libre && !t.invertido {
                t.desde += d * t.vel;
            }
        }
    }
    t.desde = t.desde.max(0.0);
}

/// Subtítulos SRT a (inicio, duración, texto).
pub fn leer_srt(s: &str) -> Vec<(f64, f64, String)> {
    let seg = |x: &str| -> Option<f64> {
        let x = x.trim().replace(',', ".");
        let mut p = x.split(':');
        Some(p.next()?.trim().parse::<f64>().ok()? * 3600.0 + p.next()?.parse::<f64>().ok()? * 60.0 + p.next()?.split_whitespace().next()?.parse::<f64>().ok()?)
    };
    let s = s.replace("\r\n", "\n");
    s.split("\n\n")
        .filter_map(|b| {
            let mut l = b.trim().trim_start_matches('\u{feff}').lines().skip_while(|l| !l.contains("-->"));
            let (a, z) = l.next()?.split_once("-->")?;
            let (a, z) = (seg(a)?, seg(z)?);
            let t: Vec<&str> = l.map(str::trim).filter(|l| !l.is_empty()).collect();
            (z > a && !t.is_empty()).then(|| (a, z - a, t.join("\n")))
        })
        .collect()
}
