//! Nodos y filtros: que el grafo haga lo que dicen los cables.
use std::sync::Arc;
use toma_uno::composicion::*;
use toma_uno::filtros::{FILTROS, efecto};
use toma_uno::modelo::*;
use toma_uno::nodos::*;
use toma_uno::ui::nodos::{PLANTILLAS, plantilla};

/// Un degradado con rojos, verdes, azules y grises.
struct Img;
impl Fuente for Img {
    fn cuadro(&mut self, _: &ClipVideo, t: f64, an: usize, al: usize) -> Option<Arc<Cuadro>> {
        let mut rgba = vec![255u8; an * al * 4];
        for (i, q) in rgba.chunks_mut(4).enumerate() {
            let (x, y) = ((i % an) as f32 / an as f32, (i / an) as f32 / al as f32);
            let c = [[230.0, 40.0, 30.0], [40.0, 200.0, 60.0], [40.0, 70.0, 220.0], [150.0, 150.0, 150.0]][(x * 3.999) as usize];
            (0..3).for_each(|k| q[k] = (c[k] * (0.3 + 0.7 * y)) as u8);
        }
        Some(Arc::new(Cuadro { t, ancho: an, alto: al, rgba }))
    }
    fn recientes(&self, _: &ClipVideo) -> Vec<Arc<Cuadro>> {
        vec![]
    }
}

fn proyecto() -> Proyecto {
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto) = (320, 180);
    let mut c = p.nuevo_video("/v.mp4".into(), Contenido::Archivo, 0.0, 5.0);
    (c.ancho, c.alto) = (320, 180);
    p.video = vec![c];
    p
}
fn cuadro(p: &Proyecto) -> Vec<u8> {
    componer(p, 1.0, 320, 180, &mut Img)
}
fn px(q: &[u8], x: usize, y: usize) -> [i32; 3] {
    std::array::from_fn(|k| q[(y * 320 + x) * 4 + k] as i32)
}
fn dif(a: &[u8], b: &[u8]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x.abs_diff(*y) as f64).sum::<f64>() / a.len() as f64
}

#[test]
fn grafo_vacio_no_cambia() {
    let mut p = proyecto();
    let a = cuadro(&p);
    p.video[0].nodos = Some(Grafo::default());
    assert!(dif(&a, &cuadro(&p)) < 0.6);
}

#[test]
fn conectar_y_quitar() {
    let mut g = Grafo::default();
    let a = g.agregar(Clase::Efecto, fx_nuevo(0, "bc:color"), 0.0, 0.0);
    let b = g.agregar(Clase::Efecto, fx_nuevo(0, "bc:duotono"), 0.0, 0.0);
    g.insertar(ENTRADA, a);
    g.insertar(a, b);
    assert!(g.llega(ENTRADA, SALIDA) && g.cables.len() == 3);
    assert!(!g.conectar(b, a, 0), "no se permiten vueltas");
    assert!(!g.conectar(SALIDA, a, 0) && !g.conectar(a, b, 7));
    g.quitar(a);
    assert!(g.cables.contains(&Cable { de: ENTRADA, a: b, puerto: 0 }) && g.cables.len() == 2);
    g.quitar(ENTRADA);
    g.quitar(SALIDA);
    assert_eq!(g.nodos.len(), 3, "entrada y salida no se quitan");
    g.acomodar();
    let x = |id| g.nodo(id).unwrap().x;
    assert!(x(ENTRADA) < x(b) && x(b) < x(SALIDA));
}

#[test]
fn mascara_calificador_y_fusion() {
    let mut p = proyecto();
    let a = cuadro(&p);
    // Dentro de la elipse se vuelve gris; en la esquina no.
    let mut g = Grafo::default();
    let mut f = fx_nuevo(0, "bc:color").unwrap();
    f.p.get_mut("sat").unwrap().v = Valor::Num(0.0);
    let e = g.agregar(Clase::Efecto, Some(f.clone()), 0.0, 0.0);
    g.insertar(ENTRADA, e);
    let m = g.agregar(Clase::Mascara, None, 0.0, 0.0);
    assert!(g.conectar(m, e, 1));
    p.video[0].nodos = Some(g.clone());
    let q = cuadro(&p);
    let gris = |c: [i32; 3]| (c[0] - c[1]).abs() < 6 && (c[1] - c[2]).abs() < 6;
    assert!(gris(px(&q, 150, 90)), "{:?}", px(&q, 150, 90));
    assert!(!gris(px(&q, 4, 4)) && (0..3).all(|k| (px(&q, 4, 4)[k] - px(&a, 4, 4)[k]).abs() < 5));
    // Calificador: solo el rojo conserva su color.
    plantilla(&mut g, 2, |t| fx_nuevo(0, t));
    p.video[0].nodos = Some(g.clone());
    let q = cuadro(&p);
    assert!(!gris(px(&q, 30, 150)) && gris(px(&q, 120, 150)) && gris(px(&q, 200, 150)), "{:?} {:?} {:?}", px(&q, 30, 150), px(&q, 120, 150), px(&q, 200, 150));
    // Fusionar al 50 %: a medio camino entre la imagen y un negro.
    let mut g = Grafo::default();
    let (n, k) = (g.agregar(Clase::Fondo, None, 0.0, 0.0), g.agregar(Clase::Fusionar, None, 0.0, 0.0));
    g.nodo_mut(n).unwrap().fx.p.get_mut("color").unwrap().v = Valor::Color([0.0, 0.0, 0.0, 1.0]);
    g.nodo_mut(k).unwrap().fx.p.get_mut("opacidad").unwrap().v = Valor::Num(50.0);
    g.conectar(ENTRADA, k, 0);
    g.conectar(n, k, 1);
    g.conectar(k, SALIDA, 0);
    p.video[0].nodos = Some(g.clone());
    let q = cuadro(&p);
    assert!((px(&q, 30, 150)[0] - px(&a, 30, 150)[0] / 2).abs() < 4);
    // Apagado, pasa el fondo; con todo el grafo apagado, la imagen.
    g.nodo_mut(k).unwrap().fx.activo = false;
    p.video[0].nodos = Some(g.clone());
    assert!(dif(&a, &cuadro(&p)) < 0.6);
    g.nodo_mut(k).unwrap().fx.activo = true;
    g.activo = false;
    p.video[0].nodos = Some(g);
    assert!(dif(&a, &cuadro(&p)) < 0.6);
}

/// Cada plantilla cambia la imagen y vuelve igual del archivo; en un clip vacío le toca a lo de abajo.
#[test]
fn plantillas_y_archivo() {
    for (i, (n, _)) in PLANTILLAS.iter().enumerate().skip(1) {
        let mut p = proyecto();
        let a = cuadro(&p);
        let mut g = Grafo::default();
        plantilla(&mut g, i, |t| fx_nuevo(0, t));
        assert!(g.llega(ENTRADA, SALIDA), "{n}");
        p.video[0].nodos = Some(g.clone());
        let d = dif(&a, &cuadro(&p));
        assert!(d > 1.0, "{n}: {d}");
        let r = std::env::temp_dir().join(format!("nodos-{}-{i}.bcraft", std::process::id()));
        toma_uno::archivo::guardar(&p, &r, None, false).unwrap();
        assert!(toma_uno::archivo::abrir(&r, &std::env::temp_dir()).unwrap() == p);
        let _ = std::fs::remove_file(r);
        p.video[0].nodos = None;
        p.capas.push(Capa { nombre: "V2".into(), visible: true, bloqueada: false });
        let mut v = p.nuevo_video("Ajuste".into(), Contenido::Ajuste, 0.0, 5.0);
        (v.capa, v.nodos) = (1, Some(g));
        p.video.push(v);
        assert!(dif(&a, &cuadro(&p)) > 1.0, "{n} en clip vacío");
    }
}

/// Todos los filtros se notan y son distintos entre sí.
#[test]
fn filtros() {
    let mut p = proyecto();
    let a = cuadro(&p);
    let mut v: Vec<(&str, Vec<u8>)> = vec![];
    for (i, f) in FILTROS.iter().enumerate() {
        p.video[0].fx = vec![efecto(9, i).unwrap()];
        let q = cuadro(&p);
        assert!(dif(&a, &q) > 1.5, "{}: {}", f.nombre, dif(&a, &q));
        for (n, o) in &v {
            assert!(dif(o, &q) > 0.8, "{} y {n} son casi iguales", f.nombre);
        }
        v.push((f.nombre, q));
    }
    println!("{} filtros", FILTROS.len());
    // En neutro no hace nada.
    p.video[0].fx = vec![fx_nuevo(3, "bc:color").unwrap()];
    assert!(dif(&a, &cuadro(&p)) < 0.6);
}

/// Los efectos propios no truenan ni dan números rotos en ningún momento, tamaño o valor extremo.
#[test]
fn efectos_propios_estables() {
    let mut img = effectcraft_raster::Image::new(96, 54);
    img.data.iter_mut().enumerate().for_each(|(i, p)| *p = [(i % 96) as f32 / 96.0, (i / 96) as f32 / 54.0, 0.4, if i % 7 == 0 { 0.5 } else { 1.0 }]);
    for d in &toma_uno::vpropios::DEFS {
        for lado in 0..3 {
            let mut f = toma_uno::vpropios::nuevo(3, d);
            for c in d.ctl {
                if let (toma_uno::vpropios::Ctl::N(k, _, a, b, ..), true) = (c, lado > 0) {
                    f.p.insert(k.to_string(), Prop::num(if lado == 1 { *a } else { *b }));
                }
            }
            for t in [0.0, 0.37, 1.9, 61.3] {
                let o = toma_uno::vpropios::aplicar(&f, t, &img, 0.1);
                assert_eq!((o.width, o.height), (96, 54));
                assert!(o.data.iter().flatten().all(|v| v.is_finite()), "{} lado {lado} en {t}", d.nombre);
            }
        }
    }
}
