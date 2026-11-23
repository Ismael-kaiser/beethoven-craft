//! Con un video real: `VIDEO=/ruta cargo test --release --test video_real -- --ignored --nocapture`.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use toma_uno::audio::{Reloj, salida_simulada};
use toma_uno::efectos::TipoFx;
use toma_uno::exportar::{Modo, exportar};
use toma_uno::medios;
use toma_uno::modelo::*;
use toma_uno::motor::{self, Almacen, Motor};
use toma_uno::composicion;
use toma_uno::video::{Lectores, VistaPrevia};

fn video() -> PathBuf {
    PathBuf::from(std::env::var("VIDEO").expect("falta VIDEO"))
}

fn proyecto() -> (Proyecto, Almacen) {
    let v = video();
    let i = medios::sondear(&v).unwrap();
    println!("sondeo: {i:?}");
    let a = Arc::new(medios::decodificar(&v).unwrap());
    let mut p = Proyecto::nuevo();
    p.pistas.clear();
    let (w, h, fps) = i.video.unwrap();
    (p.ancho, p.alto, p.fps) = (w, h, fps);
    let c = p.nuevo_video(v.clone(), Contenido::Archivo, 0.0, i.dur);
    p.video.push(c);
    let t = p.nueva_pista("Audio del video");
    let (id, total) = (p.id(), a.dur());
    p.pista(t).unwrap().clips.push(Clip {
        id, ruta: v.clone(), nombre: "v".into(), inicio: 0.0, desde: 0.0, dur: total, total,
        ganancia_db: 0.0, fade_in: 0.0, fade_out: 0.0, invertido: false, grupo: None, velocidad: 1.0,
    });
    (p, Almacen::from([(v, a)]))
}

fn sacar(p: &Proyecto, a: &Almacen, n: &str, m: Modo) -> PathBuf {
    let r = std::env::temp_dir().join(n);
    let t = Instant::now();
    exportar(p, a, &r, m, &AtomicU32::new(0), &AtomicBool::new(false)).unwrap();
    println!("{n}: exportado en {:.1} s", t.elapsed().as_secs_f32());
    r
}

fn ffprobe(r: &Path) -> String {
    let o = Command::new(medios::herramienta("ffprobe"))
        .args(["-v", "error", "-show_entries", "stream=codec_type,codec_name,width,height,r_frame_rate,sample_rate,channels,duration,nb_frames", "-of", "compact=nk=0"])
        .arg(r).output().unwrap();
    let s = String::from_utf8_lossy(&o.stdout).to_string();
    print!("{s}");
    s
}

/// Retraso de `y` respecto a `x` en muestras, por correlación (ventana de ±`m`).
fn retraso(x: &[[f32; 2]], y: &[[f32; 2]], desde: usize, n: usize, m: i64) -> i64 {
    (-m..=m)
        .max_by(|&a, &b| {
            let c = |d: i64| (0..n).map(|i| x[desde + i][0] * y[(desde as i64 + i as i64 + d) as usize][0]).sum::<f32>();
            c(a).total_cmp(&c(b))
        })
        .unwrap()
}

fn cuadro(r: &Path, t: f64) -> Vec<u8> {
    medios::ffmpeg().args(["-ss", &format!("{t:.4}"), "-i"]).arg(r)
        .args(["-frames:v", "1", "-vf", "scale=320:180", "-pix_fmt", "gray", "-f", "rawvideo", "pipe:1"])
        .output().unwrap().stdout
}
fn difer(a: &[u8], b: &[u8]) -> f64 {
    assert!(!a.is_empty() && a.len() == b.len());
    a.iter().zip(b).map(|(x, y)| (*x as f64 - *y as f64).abs()).sum::<f64>() / a.len() as f64
}

#[test]
#[ignore]
fn exportar_completo() {
    let (mut p, a) = proyecto();
    let f = p.nuevo_fx(TipoFx::Limitador);
    p.pistas[0].fx.push(f);
    let r = sacar(&p, &a, "tu-video.mp4", Modo::Video);
    let s = ffprobe(&r);
    assert!(s.contains("codec_type=video") && s.contains("codec_type=audio") && s.contains("codec_name=h264"));
    let y = medios::decodificar(&r).unwrap();
    let x = a.values().next().unwrap();
    for t in [5, 90, 180] {
        let d = retraso(&x.cuadros, &y.cuadros, t * 48000, 24000, 3000);
        println!("retraso del audio exportado en {t} s: {d} muestras ({:.2} ms)", d as f32 / 48.0);
        assert!(d.abs() < 48);
    }
    for t in [1.5, 47.3, 120.1, 180.0] {
        let d = difer(&cuadro(&video(), t), &cuadro(&r, t));
        let lejos = difer(&cuadro(&video(), t + 5.0), &cuadro(&r, t));
        println!("cuadro en {t} s: diferencia {d:.2} (contra otro momento: {lejos:.2})");
        assert!(d < 4.0);
    }
    for (n, c) in [("tu-audio.wav", "pcm_s24le"), ("tu-audio.mp3", "mp3"), ("tu-audio.flac", "flac"), ("tu-audio.m4a", "aac")] {
        let r = sacar(&p, &a, n, Modo::Audio);
        let s = ffprobe(&r);
        assert!(s.contains(c) && !s.contains("codec_type=video"));
        std::fs::remove_file(r).unwrap();
    }
    std::fs::remove_file(r).unwrap();
}

#[test]
#[ignore]
fn exportar_editado() {
    // Dos trozos del video, con hueco negro, y el audio movido y con reverberación.
    let (mut p, a) = proyecto();
    let mut b = p.video[0].clone();
    b.id = p.id();
    (p.video[0].inicio, p.video[0].desde, p.video[0].dur) = (1.0, 30.0, 4.0);
    (b.inicio, b.desde, b.dur) = (7.0, 100.0, 3.0);
    p.video.push(b);
    let c = &mut p.pistas[0].clips[0];
    (c.inicio, c.desde, c.dur) = (1.0, 30.0, 9.0);
    let f = p.nuevo_fx(TipoFx::Reverberacion);
    p.pistas[0].fx.push(f);
    let r = sacar(&p, &a, "tu-editado.mp4", Modo::Video);
    let s = ffprobe(&r);
    assert!(s.contains("1920") && s.contains("1080"));
    let d: f64 = medios::sondear(&r).unwrap().dur;
    println!("duración {d}");
    assert!((d - 10.0).abs() < 0.1);
    let medio = |t: f64| cuadro(&r, t).iter().map(|x| *x as f64).sum::<f64>() / (320.0 * 180.0);
    println!("brillo en los huecos: {:.1} y {:.1}", medio(0.5), medio(6.0));
    assert!(medio(0.5) < 18.0 && medio(6.0) < 18.0);
    // En cada punto se busca a qué momento del original se parece más el cuadro exportado.
    let fps = 24000.0 / 1001.0;
    for (n, ini, desde) in [(40.0f64, 1.0, 30.0), (100.0, 1.0, 30.0), (175.0, 7.0, 100.0), (192.0, 7.0, 100.0), (230.0, 7.0, 100.0)] {
        let e = cuadro(&r, n / fps - 0.005);
        let ideal = desde + n / fps - ini;
        let mejor = (-4..=4)
            .map(|k| ((ideal * fps).round() + k as f64) / fps)
            .min_by(|a, b| difer(&cuadro(&video(), a - 0.005), &e).total_cmp(&difer(&cuadro(&video(), b - 0.005), &e)))
            .unwrap();
        println!("cuadro {n} (original en {ideal:.3} s): corresponde a {mejor:.3} s, error {:+.0} ms", (mejor - ideal) * 1000.0);
        assert!((mejor - ideal).abs() < 0.6 / fps);
    }
    std::fs::remove_file(r).unwrap();
}

#[test]
#[ignore]
fn sincronia_de_la_vista_previa() {
    let (mut p, a) = proyecto();
    for con_fx in [false, true] {
        if con_fx {
            // Con escala, título encima y un desenfoque: se compone cuadro por cuadro.
            p.video[0].mov.escala = Prop::num(90.0);
            p.video[0].fx.push(composicion::fx_nuevo(99, "ec.blur.gaussian").unwrap());
            p.capas.push(Capa { nombre: "V2".into(), visible: true, bloqueada: false });
            let mut t = p.nuevo_video("t".into(), Contenido::Titulo(toma_uno::titulos::plantilla(0).1), 0.0, 400.0);
            t.capa = 1;
            p.video.push(t);
        }
        let p = Arc::new(p.clone());
        let motor = Arc::new(Mutex::new(Motor::default()));
        motor.lock().unwrap().aplicar(motor::preparar(&p, &a, &[]));
        let reloj = Arc::new(Mutex::new(Reloj::nuevo()));
        let fin = Arc::new(AtomicBool::new(false));
        salida_simulada(motor.clone(), reloj.clone(), fin.clone());
        let mut v = VistaPrevia::default();
        for ini in [0.0, 63.4, 150.0] {
            motor.lock().unwrap().tocando = false;
            std::thread::sleep(Duration::from_millis(30));
            motor.lock().unwrap().ir(motor::cuadros(ini));
            *reloj.lock().unwrap() = Reloj { pos: ini, instante: Instant::now(), tocando: false, desde: ini };
            let t0 = Instant::now();
            let mut vis = None;
            while vis.is_none() && t0.elapsed().as_secs() < 5 {
                if let Some(q) = v.actualizar(&p, 1, ini, false, (960, 540)) {
                    if (q.t - ini).abs() < 0.1 {
                        vis = Some(q.t);
                    }
                }
                std::thread::sleep(Duration::from_millis(4));
            }
            println!("fx={con_fx} salto a {ini} s: cuadro en {} ms", t0.elapsed().as_millis());
            motor.lock().unwrap().tocando = true;
            let t0 = Instant::now();
            let (mut errores, mut visto, mut n) = (vec![], vis, 0);
            while t0.elapsed().as_secs_f32() < 5.0 {
                let t = reloj.lock().unwrap().ahora();
                if let Some(q) = v.actualizar(&p, 1, t, true, (960, 540)) {
                    visto = Some(q.t);
                    n += 1;
                }
                if let Some(x) = visto {
                    errores.push((t - x) * 1000.0);
                }
                std::thread::sleep(Duration::from_millis(8));
            }
            errores.sort_by(f64::total_cmp);
            println!(
                "  reproducir 5 s: {n} cuadros; atraso: mediana {:.0} ms, p95 {:.0} ms, máx {:.0} ms",
                errores[errores.len() / 2], errores[errores.len() * 95 / 100], errores.last().unwrap()
            );
            assert!(n > if con_fx { 60 } else { 100 });
        }
        fin.store(true, Ordering::Relaxed);
    }
}

/// Exporta con todo: dos capas, movimiento animado, efecto, título, velocidad y transiciones.
#[test]
#[ignore]
fn exportar_compuesto() {
    let (mut p, a) = proyecto();
    let mut b = p.video[0].clone();
    b.id = p.id();
    (p.video[0].desde, p.video[0].dur) = (30.0, 4.0);
    (b.inicio, b.desde, b.dur) = (4.0, 100.0, 4.0);
    b.mov.velocidad = 2.0;
    b.entrada = Some(Transicion { tipo: "cross_dissolve".into(), dur: 1.0, p: Default::default() });
    b.salida = Some(Transicion { tipo: "dip_to_black".into(), dur: 0.5, p: Default::default() });
    p.video.push(b);
    p.capas.push(Capa { nombre: "V2".into(), visible: true, bloqueada: false });
    let mut t = p.nuevo_video("t".into(), Contenido::Titulo(toma_uno::titulos::plantilla(3).1), 1.0, 2.0);
    t.capa = 1;
    t.mov.opacidad = Prop { v: Valor::Num(100.0), claves: vec![(0.0, Valor::Num(0.0)), (0.5, Valor::Num(100.0))], curvas: vec![] };
    p.video.push(t);
    p.pistas[0].clips[0].dur = 8.0;
    p.pistas[0].clips[0].desde = 30.0;
    let r = sacar(&p, &a, "bc-compuesto.mp4", Modo::Video);
    let s = ffprobe(&r);
    assert!(s.contains("h264") && s.contains("aac") && s.contains("1920") && s.contains("24000/1001"));
    let y = medios::decodificar(&r).unwrap();
    let x = a.values().next().unwrap();
    let xs: Vec<[f32; 2]> = x.cuadros[30 * 48000..38 * 48000].to_vec();
    let d = retraso(&xs, &y.cuadros, 3 * 48000, 24000, 3000);
    println!("retraso del audio: {d} muestras");
    assert!(d.abs() < 48);
    let fps = 24000.0 / 1001.0;
    for (n, ideal, que) in [(80.0, 30.0 + 80.0 / fps, "clip 1 limpio"), (140.0, 100.0 + (140.0 / fps - 4.0) * 2.0, "clip 2 a 2x")] {
        let e = cuadro(&r, n / fps - 0.005);
        let mejor = (-4..=4).map(|k| ((ideal * fps).round() + k as f64) / fps)
            .min_by(|a, b| difer(&cuadro(&video(), a - 0.005), &e).total_cmp(&difer(&cuadro(&video(), b - 0.005), &e))).unwrap();
        println!("{que}: cuadro {n} es el de {mejor:.3} s (ideal {ideal:.3}), error {:+.0} ms, diferencia {:.2}", (mejor - ideal) * 1000.0, difer(&cuadro(&video(), mejor - 0.005), &e));
        assert!((mejor - ideal).abs() < 1.1 / fps);
    }
    // Título: de 1.0 a 1.5 aparece; a la mitad de la transición hay mezcla; al final, negro.
    // Solo el centro, donde va el texto.
    let medio = |v: Vec<u8>| v[70 * 320..110 * 320].to_vec();
    let sin = difer(&medio(cuadro(&video(), 31.0)), &medio(cuadro(&r, 1.0)));
    let con = difer(&medio(cuadro(&video(), 32.0)), &medio(cuadro(&r, 2.0)));
    println!("título: diferencia al inicio {sin:.2}, ya visible {con:.2}");
    assert!(con > sin + 2.0);
    let (ma, mb) = (difer(&cuadro(&video(), 34.5), &cuadro(&r, 4.5)), difer(&cuadro(&video(), 101.0), &cuadro(&r, 4.5)));
    println!("disolvencia a la mitad: diferencia con A {ma:.1}, con B {mb:.1}");
    assert!(ma > 2.0 && mb > 2.0);
    let fin: f64 = cuadro(&r, 7.94).iter().map(|x| *x as f64).sum::<f64>() / (320.0 * 180.0);
    println!("brillo al final: {fin:.1}");
    assert!(fin < 25.0);
    std::fs::remove_file(r).unwrap();
}

/// Cada efecto y cada transición, por el mismo camino que usa la app.
#[test]
fn efectos_y_transiciones_no_truenan() {
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto, p.fps) = (320, 180, 30.0);
    let mut t = toma_uno::titulos::plantilla(0).1;
    t.fondo = [0.2, 0.4, 0.8, 1.0];
    let c = p.nuevo_video("a".into(), Contenido::Titulo(t), 0.0, 2.0);
    let d = p.nuevo_video("b".into(), Contenido::Color([0.7, 0.3, 0.1, 1.0]), 2.0, 2.0);
    p.video = vec![c, d];
    let mut l = Lectores::nuevos(30.0, true);
    let base = composicion::componer(&p, 1.0, 320, 180, &mut l);
    std::panic::set_hook(Box::new(|_| {}));
    let (mut cambian, mut lentos) = (0, vec![]);
    for e in toma_uno::nombres::EFECTOS {
        let f = composicion::fx_nuevo(5, e.2).unwrap_or_else(|| panic!("{} no existe", e.2));
        p.video[0].fx = vec![f];
        let t0 = Instant::now();
        let o = composicion::componer(&p, 1.0, 320, 180, &mut l);
        if t0.elapsed().as_millis() > 200 {
            lentos.push(e.1);
        }
        assert_eq!(o.len(), base.len());
        cambian += (o != base) as u32;
    }
    println!("{} efectos; {cambian} cambian la imagen sin tocar nada; lentos: {lentos:?}", toma_uno::nombres::EFECTOS.len());
    p.video[0].fx.clear();
    let (a, b) = (composicion::componer(&p, 1.99, 320, 180, &mut l), composicion::componer(&p, 3.5, 320, 180, &mut l));
    let mut sin = vec![];
    for e in toma_uno::nombres::TRANSICIONES {
        p.video[1].entrada = Some(Transicion { tipo: e.2.into(), dur: 1.0, p: Default::default() });
        let o: Vec<_> = [2.3, 2.5, 2.7].iter().map(|t| composicion::componer(&p, *t, 320, 180, &mut l)).collect();
        if o.iter().all(|x| *x == a || *x == b) {
            sin.push(e.1);
        }
    }
    println!("{} transiciones; sin cuadro intermedio: {sin:?}", toma_uno::nombres::TRANSICIONES.len());
    assert!(sin.len() < 6);
}

#[test]
#[ignore]
fn salida_real_en_silencio() {
    let (p, a) = proyecto();
    let motor = Arc::new(Mutex::new(Motor::default()));
    motor.lock().unwrap().aplicar(motor::preparar(&p, &a, &[]));
    let reloj = Arc::new(Mutex::new(Reloj::nuevo()));
    let s = toma_uno::audio::abrir_salida(motor.clone(), reloj.clone(), true).unwrap();
    println!("salida: {}", s.nombre);
    let p = Arc::new(p);
    let mut v = VistaPrevia::default();
    motor.lock().unwrap().ir(motor::cuadros(20.0));
    for _ in 0..100 {
        v.actualizar(&p, 1, 20.0, false, (960, 540));
        std::thread::sleep(Duration::from_millis(4));
    }
    motor.lock().unwrap().tocando = true;
    let t0 = Instant::now();
    let (mut e, mut visto, mut salto, mut antes) = (vec![], 20.0, 0f64, (Instant::now(), 20.0));
    while t0.elapsed().as_secs_f32() < 6.0 {
        let r = *reloj.lock().unwrap();
        let t = r.ahora();
        let ahora = Instant::now();
        if r.tocando && t > 20.05 {
            salto = salto.max(((t - antes.1) - (ahora - antes.0).as_secs_f64()).abs() * 1000.0);
        }
        antes = (ahora, t);
        if let Some(q) = v.actualizar(&p, 1, t, true, (960, 540)) {
            visto = q.t;
        }
        e.push((t - visto) * 1000.0);
        std::thread::sleep(Duration::from_millis(8));
    }
    e.sort_by(f64::total_cmp);
    let avance = reloj.lock().unwrap().ahora() - 20.0;
    println!(
        "reloj: avanzó {avance:.3} s en {:.3} s; irregularidad máx {salto:.1} ms; cuadro: mediana {:.0} ms, p95 {:.0} ms",
        t0.elapsed().as_secs_f32(), e[e.len() / 2], e[e.len() * 95 / 100]
    );
    assert!((avance - t0.elapsed().as_secs_f64()).abs() < 0.3 && !s.rota.load(Ordering::Relaxed));
}

#[test]
#[ignore]
fn exportar_con_opciones() {
    use toma_uno::exportar::{Codec, Opciones, exportar_con};
    let (p, a) = proyecto();
    for (n, op, pide) in [
        ("bc-vertical.mp4", Opciones { tam: Some((1080, 1920)), tramo: Some((60.0, 63.0)), ..Default::default() }, vec!["codec_name=h264", "width=1080", "height=1920"]),
        ("bc-265.mp4", Opciones { codec: Codec::H265, tam: Some((1280, 720)), tramo: Some((60.0, 63.0)), calidad: 1 }, vec!["codec_name=hevc", "width=1280"]),
        ("bc-prores.mov", Opciones { codec: Codec::ProRes, tramo: Some((60.0, 62.0)), ..Default::default() }, vec!["codec_name=prores", "pcm_s24le"]),
    ] {
        let r = std::env::temp_dir().join(n);
        exportar_con(&p, &a, &r, Modo::Video, &op, &AtomicU32::new(0), &AtomicBool::new(false)).unwrap();
        let s = ffprobe(&r);
        for x in pide {
            assert!(s.contains(x), "{n}: falta {x}");
        }
        let (i, f) = op.tramo.unwrap();
        let d = medios::sondear(&r).unwrap().dur;
        println!("{n}: dura {d:.2}, {} KB", std::fs::metadata(&r).unwrap().len() / 1024);
        assert!((d - (f - i)).abs() < 0.1);
        // El sonido es el del tramo.
        let y = medios::decodificar(&r).unwrap();
        let x: Vec<[f32; 2]> = a.values().next().unwrap().cuadros[(i as usize * 48000)..(f as usize * 48000)].to_vec();
        let ret = retraso(&x, &y.cuadros, 30000, 24000, 3000);
        println!("{n}: retraso {ret} muestras");
        assert!(ret.abs() < 100);
        std::fs::remove_file(r).unwrap();
    }
}

/// Con un efecto pesado la vista previa baja de resolución al reproducir, y al pausar vuelve a la completa.
#[test]
#[ignore]
fn vista_previa_baja_la_calidad_si_no_alcanza() {
    let (mut p, _) = proyecto();
    p.video[0].fx.push(composicion::fx_nuevo(99, "ec.blur.directional").unwrap());
    let p = Arc::new(p);
    let mut v = VistaPrevia::default();
    let tam = (960, 540);
    let mut pedir = |t: f64, tocando: bool| loop {
        if let Some(c) = v.actualizar(&p, 1, t, tocando, tam) {
            return (c.ancho, c.alto);
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let mut anchos = vec![];
    for n in 0..12 {
        anchos.push(pedir(n as f64 / p.fps.clamp(1.0, 30.0), true).0);
    }
    println!("reproduciendo: {anchos:?}");
    assert_eq!(anchos[0], 960, "empieza en calidad completa");
    assert!(*anchos.last().unwrap() < 960, "baja cuando no alcanza: {anchos:?}");
    assert_eq!(pedir(1.0, false), tam, "en pausa vuelve a la calidad completa");
}
