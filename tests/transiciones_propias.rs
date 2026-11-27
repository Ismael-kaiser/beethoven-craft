//! Las transiciones propias (corazón, telón, tinta, humo, rayo eléctrico y panal): empiezan en el clip que sale,
//! terminan en el que entra, cambian sin saltos y se ven distinto a la mitad. `CAPTURAS=/carpeta` guarda cada una
//! a la mitad (PPM).
use effectcraft_raster::Image;
use toma_uno::composicion::{transicion, transicion_nueva};

const IDS: [&str; 6] = ["bc_corazon", "bc_telon", "bc_tinta", "bc_humo", "bc_rayo", "bc_panal"];

fn imagen(w: u32, h: u32, f: impl Fn(f32, f32) -> [f32; 4]) -> Image {
    let mut i = Image::new(w, h);
    for y in 0..h {
        for x in 0..w {
            i.data[(y * w + x) as usize] = f(x as f32 / w as f32, y as f32 / h as f32);
        }
    }
    i
}

fn diferencia(a: &Image, b: &Image) -> f32 {
    a.data.iter().zip(&b.data).map(|(p, q)| (0..4).map(|k| (p[k] - q[k]).abs()).sum::<f32>()).sum::<f32>() / a.data.len() as f32
}

#[test]
fn las_transiciones_propias() {
    let (w, h) = (320, 180);
    // Lo que sale: degradado cálido; lo que entra: rayas frías.
    let a = imagen(w, h, |x, y| [0.8 * x + 0.1, 0.3 * y + 0.1, 0.15, 1.0]);
    let b = imagen(w, h, |x, y| if ((x * 12.0) as i32 + (y * 8.0) as i32) % 2 == 0 { [0.1, 0.4, 0.8, 1.0] } else { [0.05, 0.2, 0.5, 1.0] });
    for id in IDS {
        assert!(toma_uno::nombres::TRANSICIONES.iter().any(|e| e.2 == id), "{id} está en la lista");
        let tr = transicion_nueva(id);
        let en = |p: f32| transicion(&tr, &a, &b, p, 320.0 / 1920.0);
        let (cero, uno, medio) = (en(0.0), en(1.0), en(0.5));
        assert!(diferencia(&cero, &a) < 1e-3, "{id}: empieza en lo que sale");
        assert!(diferencia(&uno, &b) < 1e-3, "{id}: termina en lo que entra");
        assert!(medio.data.iter().flatten().all(|v| v.is_finite()), "{id}: números de verdad");
        let (da, db) = (diferencia(&medio, &a), diferencia(&medio, &b));
        println!("{id}: a la mitad se aleja {da:.3} de lo que sale y {db:.3} de lo que entra");
        assert!(da > 0.03 && db > 0.03, "{id}: a la mitad es otra cosa");
        // Sin brincos cerca de las orillas.
        assert!(diferencia(&en(0.01), &a) < 0.05 && diferencia(&en(0.99), &b) < 0.05, "{id}: cambia suave en las orillas");
        if let Ok(c) = std::env::var("CAPTURAS") {
            for (k, p) in [0.3f32, 0.5, 0.7].into_iter().enumerate() {
                let i = en(p);
                let mut ppm = format!("P6 {w} {h} 255\n").into_bytes();
                for px in &i.data {
                    let a = px[3].max(1e-6);
                    ppm.extend((0..3).map(|k| ((px[k] / a).clamp(0.0, 1.0) * 255.0) as u8));
                }
                std::fs::write(format!("{c}/{id}-{k}.ppm"), ppm).unwrap();
            }
        }
    }
}
