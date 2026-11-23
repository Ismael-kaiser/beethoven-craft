//! Animación y clip vacío: que se vean de verdad en el cuadro compuesto.
use std::sync::Arc;
use toma_uno::animaciones::{ANIMACIONES, Momento, Pista, poner_en};
use toma_uno::composicion::*;
use toma_uno::curvas::Curva;
use toma_uno::modelo::*;

struct Nada;
impl Fuente for Nada {
    fn cuadro(&mut self, _: &ClipVideo, _: f64, _: usize, _: usize) -> Option<Arc<Cuadro>> {
        None
    }
    fn recientes(&self, _: &ClipVideo) -> Vec<Arc<Cuadro>> {
        vec![]
    }
}

fn proyecto() -> Proyecto {
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto, p.fps) = (640, 360, 30.0);
    p.video.clear();
    p
}

fn poner(p: &mut Proyecto, c: Contenido, capa: usize, dur: f64) -> u64 {
    while p.capas.len() <= capa {
        let n = p.capas.len() + 1;
        p.capas.push(Capa { nombre: format!("V{n}"), visible: true, bloqueada: false });
    }
    let mut k = p.nuevo_video("x".into(), c, 0.0, dur);
    k.capa = capa;
    let id = k.id;
    p.video.push(k);
    id
}

fn cuadro(p: &Proyecto, t: f64) -> Vec<u8> {
    componer(p, t, 640, 360, &mut Nada)
}

fn dif(a: &[u8], b: &[u8]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x.abs_diff(*y) as f64).sum::<f64>() / a.len() as f64
}

/// El efecto de un clip vacío le toca a todo lo de abajo, también a los textos, y solo mientras dura.
#[test]
fn clip_vacio_afecta_textos() {
    let mut p = proyecto();
    poner(&mut p, Contenido::Color([0.1, 0.2, 0.6, 1.0]), 0, 10.0);
    poner(&mut p, Contenido::Titulo(toma_uno::titulos::plantilla(3).1), 1, 10.0);
    let antes = cuadro(&p, 2.0);
    let v = poner(&mut p, Contenido::Ajuste, 2, 5.0);
    assert!(dif(&antes, &cuadro(&p, 2.0)) < 0.01, "vacío no cambia nada");
    p.clip_video(v).unwrap().fx.push(fx_nuevo(90, "bc:nocturna").unwrap());
    let con = cuadro(&p, 2.0);
    // Las letras amarillas quedan verdes: ya no hay pixeles con mucho rojo.
    let rojos = |q: &[u8]| q.chunks(4).filter(|c| c[0] > 200 && c[1] > 150).count();
    assert!(rojos(&antes) > 500 && rojos(&con) == 0, "{} {}", rojos(&antes), rojos(&con));
    assert!(dif(&antes, &cuadro(&p, 7.0)) < 0.01, "fuera de su tramo no hace nada");
    // Con recorte, solo en su recuadro.
    p.clip_video(v).unwrap().mov.recorte = [50.0, 0.0, 0.0, 0.0];
    let medio = cuadro(&p, 2.0);
    let col = |q: &[u8], x: usize| q[(180 * 640 + x) * 4..][..3].to_vec();
    assert_eq!(col(&medio, 20), col(&antes, 20));
    assert_ne!(col(&medio, 630), col(&antes, 630));
}

/// Todas las animaciones listas se pueden poner en un texto y en una figura, y cambian la imagen.
#[test]
fn animaciones_listas() {
    for a in ANIMACIONES {
        for x in [Contenido::Forma(Forma::default()), Contenido::Titulo(toma_uno::titulos::plantilla(0).1)] {
            let mut p = proyecto();
            poner(&mut p, Contenido::Color([0.05, 0.05, 0.05, 1.0]), 0, 4.0);
            let id = poner(&mut p, x.clone(), 1, 4.0);
            let mut n = 500;
            let mut pone = |p: &mut Proyecto| {
                let c = p.clip_video(id).unwrap();
                poner_en(c, a, 1.0, || {
                    n += 1;
                    n
                });
            };
            let quieto = cuadro(&p, 2.0);
            pone(&mut p);
            pone(&mut p);
            let t = match a.momento {
                Momento::Entrada => a.dur * 0.2,
                Momento::Salida => 4.0 - a.dur * 0.2,
                Momento::Enfasis => 1.0 + a.dur * 0.27,
                Momento::Bucle => a.dur * 0.3,
            };
            let d = dif(&quieto, &cuadro(&p, t));
            assert!(d > 0.03, "{} no se nota ({d})", a.nombre);
            // Los que no son bucle dejan el clip en su lugar a la mitad.
            if a.momento != Momento::Bucle {
                let m = if a.momento == Momento::Enfasis { 3.0 } else { 2.0 };
                assert!(dif(&quieto, &cuadro(&p, m)) < 0.3, "{} no regresa", a.nombre);
            }
            let c = p.clip_video(id).unwrap();
            assert!(c.fx.len() <= a.fx.len(), "{}: el efecto se repite", a.nombre);
            for q in Pista::todas(c) {
                let q = q.de(c).unwrap();
                assert!(q.claves.windows(2).all(|w| w[0].0 < w[1].0 + 1e-9), "{}", a.nombre);
                assert!(q.claves.iter().all(|k| k.1.num().is_finite()));
            }
        }
    }
}

#[test]
fn texto_por_letra() {
    let base = toma_uno::titulos::plantilla(0).1;
    let mut p = proyecto();
    let id = poner(&mut p, Contenido::Titulo(base.clone()), 0, 4.0);
    let quieto = cuadro(&p, 2.0);
    let blancos = |q: &[u8]| q.chunks(4).filter(|c| c[0] > 180).count();
    for (m, n) in MovTexto::TODOS.iter().skip(1) {
        for por in [PorTexto::Letra, PorTexto::Palabra, PorTexto::Renglon] {
            let c = p.clip_video(id).unwrap();
            c.contenido = Contenido::Titulo(Titulo { anim: Some(AnimTexto { entrada: *m, salida: *m, por, dur: 0.4, paso: 0.08, ..Default::default() }), ..base.clone() });
            assert!(dif(&quieto, &cuadro(&p, 0.02)) > 0.5, "{n} no empieza fuera");
            assert!(dif(&quieto, &cuadro(&p, 2.0)) < 1.2, "{n} no llega igual: {}", dif(&quieto, &cuadro(&p, 2.0)));
            assert!(dif(&quieto, &cuadro(&p, 3.99)) > 0.5, "{n} no sale");
        }
    }
    // Máquina de escribir: cada vez hay más letras.
    p.clip_video(id).unwrap().contenido = Contenido::Titulo(Titulo { anim: Some(AnimTexto { entrada: MovTexto::Maquina, dur: 0.01, paso: 0.2, ..Default::default() }), ..base.clone() });
    let v: Vec<usize> = [0.1, 0.5, 0.9, 1.5].iter().map(|t| blancos(&cuadro(&p, *t))).collect();
    assert!(v.windows(2).all(|w| w[1] > w[0]), "{v:?}");
    for (b, n) in BucleTexto::TODOS.iter().skip(1) {
        p.clip_video(id).unwrap().contenido = Contenido::Titulo(Titulo { anim: Some(AnimTexto { entrada: MovTexto::Ninguno, bucle: *b, ..Default::default() }), ..base.clone() });
        assert!(dif(&cuadro(&p, 1.0), &cuadro(&p, 1.17)) > 0.02, "{n} no se mueve");
    }
}

#[test]
fn figuras_y_apoyo() {
    for (t, n) in TipoForma::TODOS {
        let mut p = proyecto();
        let id = poner(&mut p, Contenido::Forma(Forma { tipo: t, color: [1.0; 4], ..Default::default() }), 0, 4.0);
        let llenos = |p: &Proyecto| cuadro(p, 1.0).chunks(4).filter(|c| c[0] > 128).count();
        let a = llenos(&p);
        assert!(a > 4000 && a < 144 * 144 + 400, "{n}: {a}");
        if let Contenido::Forma(f) = &mut p.clip_video(id).unwrap().contenido {
            f.linea = 2.0;
        }
        assert!(llenos(&p) < a * 2 / 3, "{n} con contorno");
        // Girar alrededor de una orilla lo mueve de lugar; alrededor del centro no.
        let c = p.clip_video(id).unwrap();
        c.mov.giro = Prop::num(180.0);
        c.mov.ancla = [10.0, 0.0];
        let q = cuadro(&p, 1.0);
        let cx = |q: &[u8]| {
            let v: Vec<usize> = q.chunks(4).enumerate().filter(|c| c.1[0] > 128).map(|c| c.0 % 640).collect();
            v.iter().sum::<usize>() as f64 / v.len().max(1) as f64
        };
        assert!((cx(&q) - (320.0 + 128.0)).abs() < 30.0, "{n}: {}", cx(&q));
    }
}

/// Un control de efecto se anima, se guarda en binario y vuelve igual; lo viejo se abre.
#[test]
fn efecto_animado_y_guardado() {
    let mut p = proyecto();
    poner(&mut p, Contenido::Color([0.4, 0.4, 0.4, 1.0]), 0, 4.0);
    let id = poner(&mut p, Contenido::Forma(Forma::default()), 1, 4.0);
    let c = p.clip_video(id).unwrap();
    c.fx.push(fx_nuevo(77, "ec.blur.gaussian").unwrap());
    let pista = Pista::Fx(77, "blurriness".into());
    let q = pista.de_mut(c).unwrap();
    q.claves = vec![(0.0, Valor::Num(0.0)), (2.0, Valor::Num(80.0))];
    q.poner_curva(0.0, Curva::Bezier(0.1, 0.9, 0.2, 1.0));
    assert!(q.en(0.5).num() > 50.0, "la Bézier arranca rápido");
    assert!(Pista::todas(c).contains(&pista));
    assert!(dif(&cuadro(&p, 0.0), &cuadro(&p, 2.0)) > 1.0);
    let r = std::env::temp_dir().join(format!("anim-{}.bcraft", std::process::id()));
    toma_uno::archivo::guardar(&p, &r, None, false).unwrap();
    assert!(toma_uno::archivo::abrir(&r, &std::env::temp_dir()).unwrap() == p);
    let _ = std::fs::remove_file(r);
    // Un movimiento de antes, sin ancho, alto ni apoyo.
    let mut j = serde_json::to_value(Movimiento::default()).unwrap();
    for k in ["ancho", "alto", "ancla"] {
        j.as_object_mut().unwrap().remove(k);
    }
    assert!(serde_json::from_value::<Movimiento>(j).unwrap() == Movimiento::default());
}
