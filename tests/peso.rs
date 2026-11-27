//! Exportar para que quepa en un peso (Discord): el archivo nunca pasa del límite y se ve lo mejor que se puede.
use std::sync::atomic::{AtomicBool, AtomicU32};
use toma_uno::exportar::{DISCORD, Modo, Opciones, ajustar_peso, exportar_con, plan_peso};
use toma_uno::medios;
use toma_uno::modelo::*;

#[test]
fn el_plan_baja_la_imagen_si_hay_pocos_bits() {
    // Un minuto en 1080p60 para Discord: 720p a 30 cuadros.
    let (v, a, alto, fps) = plan_peso(DISCORD, 60.0, 1920, 1080, 60.0);
    println!("60 s: {v} kbps de video, {a} de audio, alto {alto:?}, {fps:?} cps");
    assert!(v > 1000 && v < 1300 && alto == Some(720) && fps == Some(30.0));
    // Diez segundos caben en 1080p tal cual.
    let (v, _, alto, fps) = plan_peso(DISCORD, 10.0, 1920, 1080, 30.0);
    assert!(v > 6000 && alto.is_none() && fps.is_none());
    // Cinco minutos: chico, pero se puede ver.
    let (v, _, alto, _) = plan_peso(DISCORD, 300.0, 1920, 1080, 30.0);
    assert!(v >= 150 && alto == Some(360), "{v} {alto:?}");
}

#[test]
fn un_video_pesado_cabe() {
    let dir = std::env::temp_dir().join(format!("bc-peso-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let base = dir.join("base.mp4");
    // 20 s de 1080p con mucho ruido (lo más difícil de comprimir) y un tono.
    let ok = medios::ffmpeg()
        .args(["-y", "-v", "error", "-f", "lavfi", "-i", "testsrc2=s=1920x1080:r=30:d=20,noise=alls=40:allf=t", "-f", "lavfi", "-i", "sine=f=440:d=20"])
        .args(["-c:v", "libx264", "-preset", "ultrafast", "-crf", "12", "-c:a", "aac", "-shortest"])
        .arg(&base)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let antes = std::fs::metadata(&base).unwrap().len();
    let max = 2 * 1024 * 1024;
    let sale = dir.join("discord.mp4");
    ajustar_peso(&base, &sale, max, &AtomicU32::new(0), &AtomicBool::new(false)).unwrap();
    let pesa = std::fs::metadata(&sale).unwrap().len();
    let i = medios::sondear(&sale).unwrap();
    println!("de {:.1} MB a {:.2} MB (límite 2 MB), {:?}, {:.1} s", antes as f64 / 1048576.0, pesa as f64 / 1048576.0, i.video, i.dur);
    assert!(antes > 4 * max, "el de prueba sí pesa");
    assert!(pesa <= max, "no pasa del límite");
    assert!(pesa > max / 2, "y aprovecha lo que hay");
    assert!(i.audio && (i.dur - 20.0).abs() < 0.3);
    assert_eq!(i.video.map(|v| v.1), Some(720), "con pocos bits, 720p");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn lo_que_ya_cabe_no_se_toca() {
    let dir = std::env::temp_dir().join(format!("bc-peso-cabe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto, p.fps) = (640, 360, 30.0);
    let c = p.nuevo_video("".into(), Contenido::Color([0.2, 0.4, 0.8, 1.0]), 0.0, 2.0);
    p.video.push(c);
    let s = dir.join("chico.mp4");
    let op = Opciones { peso_max: Some(DISCORD), ..Default::default() };
    exportar_con(&p, &Default::default(), &s, Modo::Video, &op, &AtomicU32::new(0), &AtomicBool::new(false)).unwrap();
    let pesa = std::fs::metadata(&s).unwrap().len();
    assert!(pesa > 0 && pesa <= DISCORD);
    assert!(std::fs::read_dir(&dir).unwrap().count() == 1, "no deja archivos de paso");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn la_tarjeta_de_discord() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| toma_uno::ui::App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    {
        let a = h.state_mut();
        a.inicio.visible = false;
        let c = a.p.nuevo_video("".into(), Contenido::Color([0.2, 0.4, 0.8, 1.0]), 0.0, 45.0);
        a.p.video.push(c);
        a.exportar_op = Some(toma_uno::ui::exportacion::Panel::nuevo(None));
    }
    h.run_steps(3);
    h.get_all_by(|n| n.label().is_some_and(|l| l.starts_with("Discord"))).next().unwrap().click();
    h.run_steps(3);
    assert_eq!(h.state().exportar_op.as_ref().unwrap().op.peso_max, Some(DISCORD));
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/exportar-discord.png")).unwrap();
    }
}
