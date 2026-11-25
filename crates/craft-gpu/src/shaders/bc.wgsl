// Efectos de Beethoven Craft (`bc:…`), los mismos de `src/vpropios.rs`, con sus mismas fórmulas.
// f[0..9]: números del efecto (`A(k)`); f[10]: color; f[11]: ancho, alto, escala y mezcla. u[3]: enteros.

var<private> W: f32;
var<private> H: f32;
var<private> ES: f32;
var<private> CX: f32;
var<private> CY: f32;
const TAU: f32 = 6.283185307179586;

fn A(k: i32) -> f32 { return P.f[k / 4][k % 4]; }
fn N(k: i32) -> i32 { return bitcast<i32>(P.u[3][k]); }
fn C1() -> vec4<f32> { return P.f[10]; }

fn recto(p: vec4<f32>) -> vec4<f32> {
    if (p.w > 0.0) { return vec4<f32>(p.xyz / p.w, p.w); }
    return vec4<f32>(0.0);
}
fn px(x: i32, y: i32) -> vec4<f32> {
    let d = vec2<i32>(textureDimensions(src));
    return recto(textureLoad(src, vec2<i32>(clamp(x, 0, d.x - 1), clamp(y, 0, d.y - 1)), 0));
}
fn en(x0: f32, y0: f32) -> vec4<f32> {
    let x = x0 - 0.5;
    let y = y0 - 0.5;
    let i = floor(x);
    let j = floor(y);
    let u = x - i;
    let v = y - j;
    let a = i32(i);
    let b = i32(j);
    return mix(mix(px(a, b), px(a + 1, b), u), mix(px(a, b + 1), px(a + 1, b + 1), u), v);
}
// De la imagen auxiliar (luces), que es opaca.
fn ax(x: i32, y: i32) -> vec4<f32> {
    let d = vec2<i32>(textureDimensions(aux));
    return textureLoad(aux, vec2<i32>(clamp(x, 0, d.x - 1), clamp(y, 0, d.y - 1)), 0);
}
fn en_aux(x0: f32, y0: f32) -> vec4<f32> {
    let x = x0 - 0.5;
    let y = y0 - 0.5;
    let i = floor(x);
    let j = floor(y);
    let u = x - i;
    let v = y - j;
    let a = i32(i);
    let b = i32(j);
    return mix(mix(ax(a, b), ax(a + 1, b), u), mix(ax(a, b + 1), ax(a + 1, b + 1), u), v);
}
fn resto(a: f32, n: f32) -> f32 { return a - n * floor(a / n); }
fn frac(x: f32) -> f32 { return x - trunc(x); }
fn reflejo(v: f32, n: f32) -> f32 {
    let m = resto(v, 2.0 * n);
    return select(m, 2.0 * n - m, m > n);
}
fn espejo(x: f32, y: f32) -> vec4<f32> { return en(reflejo(x, W), reflejo(y, H)); }
fn luz(p: vec4<f32>) -> f32 { return 0.2126 * p.x + 0.7152 * p.y + 0.0722 * p.z; }
fn escalon(a: f32, b: f32, x: f32) -> f32 {
    let t = clamp((x - a) / (b - a), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}
fn azar(a: i32, b: i32) -> f32 {
    var h = (bitcast<u32>(a) * 0x9E3779B1u) ^ (bitcast<u32>(b) * 0x85EBCA77u);
    h ^= h >> 15u;
    h = h * 0x2C1B3C6Du;
    h ^= h >> 13u;
    h = h * 0x297A2D39u;
    h ^= h >> 16u;
    return f32(h >> 8u) / 16777216.0;
}
fn ruido(x: f32, y: f32) -> f32 {
    let i = floor(x);
    let j = floor(y);
    let tu = x - i;
    let tv = y - j;
    let u = tu * tu * (3.0 - 2.0 * tu);
    let v = tv * tv * (3.0 - 2.0 * tv);
    let a = i32(i);
    let b = i32(j);
    return mix(mix(azar(a, b), azar(a + 1, b), u), mix(azar(a, b + 1), azar(a + 1, b + 1), u), v);
}
fn nubes(x: f32, y: f32) -> f32 {
    return 0.5 * ruido(x, y) + 0.25 * ruido(x * 2.03 + 7.0, y * 2.03) + 0.15 * ruido(x * 4.1, y * 4.1 + 3.0) + 0.1 * ruido(x * 8.3, y * 8.3);
}
fn sobre(a: vec4<f32>, c: vec4<f32>, k0: f32) -> vec4<f32> {
    let k = clamp(k0, 0.0, 1.0);
    return vec4<f32>(mix(a.xyz, c.xyz, k), max(a.w, k));
}
fn pantalla(p: vec4<f32>, l: vec3<f32>) -> vec4<f32> {
    let u = vec3<f32>(1.0);
    return vec4<f32>(u - (u - clamp(p.xyz, vec3<f32>(0.0), u)) * (u - clamp(l, vec3<f32>(0.0), u)), p.w);
}
fn arcoiris(h: f32) -> vec4<f32> {
    let d = vec3<f32>(h, h + 2.0 / 3.0, h + 1.0 / 3.0);
    let r = (d - floor(d)) * 6.0 - 3.0;
    return vec4<f32>(clamp(abs(r), vec3<f32>(1.0), vec3<f32>(2.0)) - 1.0, 1.0);
}
fn girar_tono(p: vec4<f32>, ang: f32) -> vec4<f32> {
    let s = sin(ang);
    let c = cos(ang);
    let k = (1.0 - c) / 3.0;
    let q = s / 1.7320508;
    let o = vec3<f32>(
        p.x * (c + k) + p.y * (k - q) + p.z * (k + q),
        p.x * (k + q) + p.y * (c + k) + p.z * (k - q),
        p.x * (k - q) + p.y * (k + q) + p.z * (c + k));
    return vec4<f32>(clamp(o, vec3<f32>(0.0), vec3<f32>(1.0)), p.w);
}

fn E_color(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0..2 sombras, 3..5 medios, 6..8 luces, 9 exp, 10 con, 11 sat, 12 vib, 13 temp, 14 tinte, 15 giro, 16 des, 17 vin, 18 grano
    var c = vec3<f32>(p.x * A(9) * (1.0 + A(13)), p.y * A(9) * (1.0 + A(14)), p.z * A(9) * (1.0 - A(13)));
    c = (c - 0.45) * A(10) + 0.45;
    var l = clamp(luz(vec4<f32>(c, 1.0)), 0.0, 1.0);
    let ws = (1.0 - l) * (1.0 - l);
    let wl = l * l;
    let wm = 1.0 - ws - wl;
    c += vec3<f32>(A(0), A(1), A(2)) * ws + vec3<f32>(A(3), A(4), A(5)) * wm * 1.4 + vec3<f32>(A(6), A(7), A(8)) * wl;
    l = luz(vec4<f32>(c, 1.0));
    let mx = max(c.x, max(c.y, c.z));
    let mn = min(c.x, min(c.y, c.z));
    let s = A(11) * (1.0 + A(12) * (1.0 - clamp(mx - mn, 0.0, 1.0)));
    c = vec3<f32>(l) + (c - l) * s;
    if (A(15) != 0.0) { c = girar_tono(vec4<f32>(c, 1.0), A(15)).xyz; }
    if (A(16) > 0.0) { c = max(c, vec3<f32>(0.0)) * (1.0 - 0.28 * A(16)) + 0.16 * A(16); }
    if (A(17) > 0.0) {
        let u = (x - CX) / CX;
        let v = (y - CY) / CY;
        c *= 1.0 - A(17) * 0.85 * escalon(0.25, 1.05, u * u * 0.5 + v * v * 0.5);
    }
    if (A(18) > 0.0) {
        let e = max(ES, 0.5);
        c += (azar(i32(x / e) + N(0) * 97, i32(y / e)) - 0.5) * A(18) * 0.35;
    }
    return vec4<f32>(c, p.w);
}
fn E_zoom(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    return en(CX + (x - CX) / A(0), CY + (y - CY) / A(0));
}
fn E_destello(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    if (p.w > 0.0) { return vec4<f32>(sobre(p, C1(), A(0)).xyz, p.w); }
    return p;
}
fn E_tono(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    return girar_tono(p, A(0));
}
fn E_sacudida(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    return espejo(CX + (x - CX) / A(2) + A(0), CY + (y - CY) / A(2) + A(1));
}
fn E_rgb(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(en(x - A(0), y).x, p.y, en(x + A(0), y).z, p.w);
}
fn E_vueltas(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let m = N(0);
    return en(select(x, W - x, m == 1 || m == 2), select(y, H - y, m >= 2));
}
fn E_tv(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let cu = A(0);
    let li = A(1);
    let alto = max(H / 240.0, 1.5);
    let u = (x - CX) / CX;
    let v = (y - CY) / CY;
    let u2 = u * (1.0 + cu * v * v);
    let v2 = v * (1.0 + cu * u * u);
    if (abs(u2) > 1.0 || abs(v2) > 1.0) { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    let sx = CX + u2 * CX;
    let sy = CY + v2 * CY;
    let d = 0.0012 * W;
    let m0 = en(sx, sy);
    var q = vec4<f32>(en(sx - d, sy).x, m0.y, en(sx + d, sy).z, m0.w);
    let l = 1.0 - li * 0.5 * (0.5 + 0.5 * cos(sy / alto * 3.14159265));
    let o = max(1.0 - A(2) * 0.45 * (u * u + v * v), 0.0);
    let m = i32(x / alto) % 3;
    for (var k = 0; k < 3; k++) {
        let fosf = 1.0 - li * 0.22 * select(0.0, 1.0, m != k);
        q[k] = q[k] * l * o * fosf * 1.22;
    }
    return q;
}
fn E_cinta(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 k, 1 banda; N0 cua
    let k = A(0);
    let fila = i32(y / max(H / 270.0, 1.0));
    let tem = (azar(fila, N(0)) - 0.5) * 0.004 * W * k;
    let eb = (y - A(1)) / (0.05 * H);
    let en_banda = exp(-eb * eb);
    let dx = tem + en_banda * k * W * 0.05 * (ruido(y * 0.05, f32(N(0))) - 0.5) * 2.0;
    let q = 0.004 * W * k;
    let m = en(x + dx, y);
    var o = vec4<f32>(en(x + dx - q, y).x, m.y, en(x + dx + q, y).z, m.w);
    let r = azar(i32(x / 2.0) + N(0) * 131, fila);
    if (r > 1.0 - 0.18 * en_banda * min(k, 1.0)) { o = vec4<f32>(0.95, 0.95, 0.95, 1.0); }
    let l = luz(o);
    return vec4<f32>(o.xyz + (vec3<f32>(l) - o.xyz) * 0.2 * min(k, 1.0) + (r - 0.5) * 0.06 * k, o.w);
}
fn E_nocturna(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let d = length(vec2<f32>(x - CX, y - CY)) / (0.62 * W);
    let l = pow(max(luz(p) * A(0), 0.0), 0.8) + (azar(i32(x) + N(0) * 57, i32(y)) - 0.5) * A(1) * 0.6;
    let v = l * (1.0 - escalon(0.65, 1.0, d));
    return vec4<f32>(C1().xyz * v, p.w);
}
fn E_termica(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    var pal = array<vec3<f32>, 6>(vec3<f32>(0.0, 0.0, 0.12), vec3<f32>(0.22, 0.0, 0.55), vec3<f32>(0.78, 0.08, 0.5), vec3<f32>(0.98, 0.4, 0.1), vec3<f32>(1.0, 0.82, 0.15), vec3<f32>(1.0, 1.0, 0.92));
    let v = clamp(luz(p), 0.0, 1.0) * 5.0;
    let i = min(i32(v), 4);
    return vec4<f32>(mix(pal[i], pal[i + 1], v - f32(i)), p.w);
}
fn E_consola(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    var pal = array<vec3<f32>, 4>(vec3<f32>(0.09, 0.17, 0.14), vec3<f32>(0.22, 0.38, 0.27), vec3<f32>(0.5, 0.62, 0.3), vec3<f32>(0.8, 0.85, 0.55));
    var tr = array<f32, 4>(0.0, 0.5, 0.75, 0.25);
    let t = A(0);
    let i = floor(x / t);
    let j = floor(y / t);
    let q = en((i + 0.5) * t, (j + 0.5) * t);
    let v = i32(clamp(floor(luz(q) * 3.0 + tr[(i32(j) & 1) * 2 + (i32(i) & 1)] * 0.9), 0.0, 3.0));
    return vec4<f32>(pal[v], p.w);
}
fn E_barras(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    if (y < A(0) || y > H - A(0)) { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    return p;
}
fn E_marco(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let x0 = A(0);
    let x1 = W - x0;
    let y1 = H - A(1);
    if (x < x0 || x > x1 || y < x0 || y > y1) { return vec4<f32>(C1().xyz, 1.0); }
    let d = min(min(x - x0, x1 - x), min(y - x0, y1 - y));
    let q = en((x - x0) / (x1 - x0) * W, (y - x0) / (y1 - x0) * H);
    return vec4<f32>(q.xyz * (0.65 + 0.35 * escalon(0.0, 0.02 * H, d)), 1.0);
}
fn E_puntos(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let t = A(0);
    let si = A(1);
    let co = A(2);
    let u = (x * co + y * si) / t;
    let v = (-x * si + y * co) / t;
    let i = floor(u) + 0.5;
    let j = floor(v) + 0.5;
    let q = en((i * co - j * si) * t, (i * si + j * co) * t);
    let l = max(luz(q), 0.02);
    let r = sqrt(max(1.0 - l * 0.85, 0.0)) * 0.66;
    let k = escalon(r + 0.08, r - 0.08, length(vec2<f32>(u - i, v - j)));
    return mix(vec4<f32>(0.97, 0.95, 0.9, p.w), vec4<f32>(q.xyz / l * 0.45, p.w), k);
}
fn E_manga(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let ti = A(0);
    let tr = A(1);
    let o = max(1.5 * ES, 1.0);
    let gx = luz(en(x + o, y)) - luz(en(x - o, y));
    let gy = luz(en(x, y + o)) - luz(en(x, y - o));
    let borde = escalon(0.3 - 0.26 * ti, 0.4 - 0.26 * ti, length(vec2<f32>(gx, gy)));
    let b = luz(en(x, y));
    var v = 1.0;
    if (b < 0.22 + 0.1 * ti) {
        v = 0.0;
    } else if (b < 0.55) {
        let i = (x + y) / tr;
        let j = (x - y) / tr;
        let d = length(vec2<f32>(i - floor(i) - 0.5, j - floor(j) - 0.5));
        let m = 0.6 * max(0.62 - b, 0.1);
        v = escalon(m, m + 0.14, d);
    }
    v = v * (1.0 - borde);
    return vec4<f32>(v, v, v, p.w);
}
fn E_duotono(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let o = mix(P.f[8], P.f[9], escalon(0.02, 0.95, luz(p)));
    return vec4<f32>(o.xyz, p.w);
}
fn E_neblina(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let v = nubes(x / H * 2.6 + A(1) * 0.12, y / H * 3.2 + A(1) * 0.03);
    let m = escalon(0.25, 0.8, v) * (0.45 + 0.55 * y / H) * A(0) * 1.3;
    return vec4<f32>(sobre(p, C1(), m).xyz, p.w);
}
fn E_velocidad(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 cant, 1 hueco; N0 paso, N1 semilla
    let u = (x - CX) / CX;
    let v = (y - CY) / CY;
    let r = length(vec2<f32>(u, v));
    let g = (atan2(v, u) / TAU + 0.5) * 180.0;
    let sec = i32(g);
    if (azar(sec, N(0) + N(1)) > A(0)) { return p; }
    let ini = A(1) + 0.5 * azar(sec, N(0) + 77);
    let f = frac(g) - 0.5;
    let ancho = 0.5 * escalon(ini, ini + 0.9, r);
    return sobre(p, C1(), escalon(ancho, ancho * 0.6, abs(f)) * escalon(ini, ini + 0.08, r));
}
fn E_prisma(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    var o = p * 0.5;
    let n = N(0);
    for (var i = 0; i < n; i++) {
        let a = TAU * f32(i) / f32(n) + A(1);
        o += espejo(x + cos(a) * A(0), y + sin(a) * A(0)) * 0.5 / f32(n);
    }
    return o;
}
fn E_tunel(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 paso, 1 giro, 2 f0
    let u = x - CX;
    let v = y - CY;
    var z = pow(A(0), -A(2));
    var mejor = vec4<f32>(0.0);
    for (var k = 0; k < 10; k++) {
        let g = A(1) * (f32(k) - A(2));
        let si = sin(g);
        let co = cos(g);
        let a = (u * co + v * si) / z;
        let b = (-u * si + v * co) / z;
        if (abs(a) > CX || abs(b) > CY) { break; }
        let q = en(CX + a, CY + b);
        mejor = vec4<f32>(q.xyz * (1.0 - 0.04 * f32(k)), max(q.w, mejor.w));
        z = z * A(0);
    }
    return mejor;
}
fn E_agua(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let k = A(0);
    let ta = A(1);
    let tv = A(2);
    let dx = k * (sin(y / ta * 6.28 + tv * 2.1) + 0.5 * sin(y / ta * 11.0 + x / ta * 2.0 - tv * 1.3));
    let dy = k * (cos(x / ta * 5.1 + tv * 1.7) + 0.5 * sin(x / ta * 9.3 + tv * 2.6));
    let q = espejo(x + dx, y + dy);
    let l = 1.0 + 0.35 * (nubes(x / ta * 1.2 + tv * 0.5, y / ta * 1.2 - tv * 0.3) - 0.5);
    return vec4<f32>(q.x * l * 0.88, q.y * l * 0.98, q.z * l * 1.05, q.w);
}
fn E_halo(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let q = en_aux(x * 0.25, y * 0.25);
    return pantalla(p, q.xyz * A(0) * C1().xyz);
}
fn E_foco(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 ox, 1 oy, 2 z, 3 cant
    let d = length(vec2<f32>((x - A(0)) / W, (y - A(1)) / H)) * 1.6;
    let a = escalon(A(2) * 0.6, A(2) * 0.6 + 0.45, d) * min(A(3) * 4.0, 1.0);
    return mix(p, vec4<f32>(en_aux(x * 0.25, y * 0.25).xyz, p.w), a);
}
fn E_vidrio(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 ta, 1 k, 2 se, 3 co
    let u = (x - CX) * A(3) + (y - CY) * A(2);
    let c = resto(u / A(0), 1.0) - 0.5;
    let des = -c * A(0) * 2.6 * A(1) + sin(c * 9.0) * A(0) * 0.18 * A(1);
    let q = espejo(x + des * A(3), y + des * A(2));
    let b = 1.0 + A(1) * (0.16 * max(0.5 - abs(c - 0.18) * 2.0, 0.0) - 0.12 * escalon(0.36, 0.5, abs(c)));
    return vec4<f32>(q.xyz * b, q.w);
}
fn E_barrido(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 se, 1 co, 2 u0, 3 an, 4 g
    let u = ((x - CX) * A(1) + (y - CY) * A(0) - A(2)) / max(A(3), 1.0);
    let l = exp(-u * u * 2.5) * A(4) * (0.25 + 0.75 * luz(p));
    return pantalla(p, C1().xyz * l);
}
fn E_degradado(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 se, 1 co, 2 largo, 3 g
    let k = clamp(((x - CX) * A(1) + (y - CY) * A(0)) / A(2) + 0.5, 0.0, 1.0);
    let c = mix(P.f[8], P.f[9], k);
    var q = p;
    for (var j = 0; j < 3; j++) {
        let v = p[j];
        if (c[j] < 0.5) { q[j] = v - (1.0 - 2.0 * c[j]) * v * (1.0 - v); } else { q[j] = v + (2.0 * c[j] - 1.0) * (sqrt(max(v, 0.0)) - v); }
    }
    return clamp(mix(p, q, min(A(3) * 2.0, 1.0)), vec4<f32>(0.0), vec4<f32>(1.0));
}
fn E_lluvia(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 cant, 1 incl, 2 tv
    var tam = array<f32, 3>(0.010, 0.016, 0.026);
    var ve = array<f32, 3>(1.7, 2.4, 3.2);
    var al = array<f32, 3>(0.28, 0.38, 0.50);
    var l = 0.0;
    for (var c = 0; c < 3; c++) {
        let an = tam[c] * H;
        let u = (x + y * A(1)) / an;
        let i = i32(floor(u));
        if (azar(i, 31 + c) > 0.7 * A(0)) { continue; }
        let largo = an * (7.0 + 5.0 * azar(i, 5 + c));
        let ciclo = H + largo * 3.0;
        let yy = resto(y - A(2) * ve[c] * H * (0.8 + 0.4 * azar(i, 9 + c)) - azar(i, 17 + c) * ciclo, ciclo);
        if (yy < largo) {
            let d = abs(frac(u) - 0.5) * an;
            l += al[c] * (yy / largo) * (1.0 - escalon(0.8 * ES, 2.4 * ES, d));
        }
    }
    return pantalla(p, vec3<f32>(l * 0.9, l * 0.95, l));
}
fn E_radial(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    var q = vec4<f32>(0.0);
    let r = azar(i32(x), i32(y));
    for (var i = 0; i < 18; i++) {
        let z = 1.0 - A(0) * (f32(i) + r) / 18.0;
        q += en(A(1) + (x - A(1)) * z, A(2) + (y - A(2)) * z) / 18.0;
    }
    return vec4<f32>(q.xyz, max(p.w, q.w));
}
fn E_caleidoscopio(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 paso, 1 giro, 2 medio giro
    let u = x - CX;
    let v = y - CY;
    let r = length(vec2<f32>(u, v));
    var a = resto(atan2(v, u) + A(1), A(0));
    a = select(a, A(0) - a, a > A(0) / 2.0) - A(1) * 0.5;
    return espejo(CX + r * cos(a), CY + r * sin(a));
}
fn E_pendulo(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 se, 1 co, 2 z
    let u = (x - CX) * A(2);
    let v = (y - CY) * A(2);
    return espejo(CX + u * A(1) + v * A(0), CY - u * A(0) + v * A(1));
}
fn E_rebote(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    return espejo(CX + (x - CX) / A(0), H + (y - H) * A(0) + A(1));
}
fn E_remolino(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let u = x - CX;
    let v = y - CY;
    let r = length(vec2<f32>(u, v));
    if (r >= A(1)) { return p; }
    let k = 1.0 - r / A(1);
    let g = A(0) * k * k;
    return espejo(CX + u * cos(g) - v * sin(g), CY + u * sin(g) + v * cos(g));
}
fn E_onda(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 ox, 1 oy, 2 r0, 3 an, 4 g
    let a = x - A(0);
    let b = y - A(1);
    let r = max(length(vec2<f32>(a, b)), 1e-3);
    let k = (r - A(2)) / A(3);
    if (abs(k) > 2.5) { return p; }
    let d = sin(k * 2.6) * exp(-k * k) * A(4);
    let q = espejo(x - a / r * d, y - b / r * d);
    return vec4<f32>(q.xyz * (1.0 + 0.35 * d / H * 12.0), q.w);
}
fn E_sismo(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 dx, 1 dy, 2 c, 3 z
    let u = CX + (x - CX) * A(3) + A(0);
    let v = CY + (y - CY) * A(3) + A(1);
    let b = espejo(u, v);
    return vec4<f32>(espejo(u - A(2), v).x, b.y, espejo(u + A(2), v).z, b.w);
}
fn E_azulejos(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let u = x / W * A(0);
    let v = y / H * A(0);
    let m = N(0) == 1;
    let fu = select(frac(u), 1.0 - frac(u), m && i32(floor(u)) % 2 == 1);
    let fv = select(frac(v), 1.0 - frac(v), m && i32(floor(v)) % 2 == 1);
    return en(fu * W, fv * H);
}
fn E_espejo(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    var u = x;
    var v = y;
    let m = N(0);
    if (m == 0 || m > 3) { u = select(x, W - x, x > CX); }
    if (m == 1) { u = select(x, W - x, x < CX); }
    if (m == 2 || m > 3) { v = select(y, H - y, y > CY); }
    if (m == 3) { v = select(y, H - y, y < CY); }
    return en(u, v);
}
fn E_giro3d(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 rx, 1 ry, 2 z
    let razon = W / H;
    let mx = (x - CX) / H / A(2);
    let my = (y - CY) / H / A(2);
    let sx = sin(A(0));
    let cx = cos(A(0));
    let sy = sin(A(1));
    let cy = cos(A(1));
    let n = vec3<f32>(cx * sy, -sx, cx * cy);
    let den = mx * n.x + my * n.y + 1.9 * n.z;
    if (abs(den) < 1e-4) { return vec4<f32>(0.0); }
    let k = 1.9 * n.z / den;
    if (k <= 0.0) { return vec4<f32>(0.0); }
    let q = vec3<f32>(mx * k, my * k, 1.9 * k - 1.9);
    let u = q.x * cy + q.z * -sy;
    let v = q.x * sx * sy + q.y * cx + q.z * sx * cy;
    if (abs(u) > razon / 2.0 || abs(v) > 0.5) { return vec4<f32>(0.0); }
    return en((u / razon + 0.5) * W, (v + 0.5) * H);
}
fn E_cubo(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 se, 1 co
    let mitad = W / H / 2.0;
    let cam = 2.2 + mitad;
    let dx = (x - CX) / H / (cam - mitad) * 1.3;
    let dy = (y - CY) / H / (cam - mitad) * 1.3;
    let o = vec2<f32>(A(0) * -cam, A(1) * -cam);
    let dir = vec2<f32>(A(1) * dx + A(0), -A(0) * dx + A(1));
    var caras = array<vec2<f32>, 4>(vec2<f32>(0.0, -1.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(-1.0, 0.0));
    var mk = 1e30;
    var mejor = vec4<f32>(0.0);
    for (var i = 0; i < 4; i++) {
        let n = caras[i];
        let den = dot(dir, n);
        if (den >= -1e-4) { continue; }
        let k = (mitad - dot(o, n)) / den;
        let q = o + dir * k;
        let py = dy * k;
        let u = -q.x * n.y + q.y * n.x;
        if (k > 0.0 && k < mk && abs(u) <= mitad && abs(py) <= 0.5) {
            let l = (0.45 + 0.55 * (-den / length(dir))) * select(0.94, 1.0, i % 2 == 0);
            let c = en((u + mitad) * H, (py + 0.5) * H);
            mk = k;
            mejor = vec4<f32>(c.xyz * l, c.w);
        }
    }
    return mejor;
}
fn E_bandera(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 k, 1 ta, 2 tv
    let f = (x / A(1) * 6.283 - A(2) * 4.0) + y / H * 1.4;
    let a = A(0) * (0.35 + 0.65 * x / W);
    let q = espejo(x + cos(f) * a * W * 0.012, y + sin(f) * a * H * 0.07);
    return vec4<f32>(q.xyz * (1.0 + cos(f) * a * 0.45), q.w);
}
fn E_lente(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 ox, 1 oy, 2 r0, 3 k
    let u = x - A(0);
    let v = y - A(1);
    let r = length(vec2<f32>(u, v)) / max(A(2), 1.0);
    if (r >= 1.0) { return p; }
    let z = pow(max(r, 1e-3), select(A(3) * 0.6, A(3) * 1.6, A(3) >= 0.0));
    return espejo(A(0) + u * z, A(1) + v * z);
}
fn E_gotas(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 ta, 1 cant, 2 tv
    var q = p;
    for (var capa = 0; capa < 2; capa++) {
        let t2 = A(0) * (1.0 - 0.45 * f32(capa));
        let baja = A(2) * H * 0.05 * f32(capa);
        let i = i32(floor(x / t2));
        let j = i32(floor((y - baja) / t2));
        if (azar(i + capa * 57, j) > 0.5 * A(1)) { continue; }
        let ox = (f32(i) + 0.3 + 0.4 * azar(i, j + 9)) * t2;
        let oy = (f32(j) + 0.3 + 0.4 * azar(i + 5, j)) * t2 + baja;
        let r = t2 * (0.16 + 0.22 * azar(i + 2, j + 3));
        let u = (x - ox) / r;
        let v = (y - oy) / (r * 1.15);
        let d = u * u + v * v;
        if (d < 1.0) {
            let f = 1.0 - d;
            let z = espejo(ox - u * r * 5.0 * sqrt(f), oy - v * r * 5.0 * sqrt(f));
            let l = 1.0 + 0.5 * (-(u + 0.4) * 0.6 - (v + 0.4) * 0.8) * (1.0 - f);
            q = mix(q, vec4<f32>(z.xyz * l, p.w), escalon(0.0, 0.2, f));
        }
    }
    return q;
}
fn E_linterna(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 r0, 1 osc, 2 ox, 3 oy
    let d = length(vec2<f32>(x - A(2), y - A(3)));
    let l = 1.0 - escalon(A(0) * 0.45, A(0) * 1.25, d);
    let k = (1.0 - A(1)) + A(1) * l * 1.08;
    return vec4<f32>(p.x * k, p.y * k * 0.99, p.z * k * 0.94, p.w);
}
fn E_discoteca(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 g, 1 n, 2 tv
    var l = vec3<f32>(0.0);
    for (var lado = 0; lado < 2; lado++) {
        let ox = select(W * 0.9, W * 0.1, lado == 0);
        let dir = select(-1.0, 1.0, lado == 0);
        let v = y + H * 0.12;
        let a = atan2(x - ox, v) * A(1) / 1.9 + dir * A(2) * 1.7 + f32(lado);
        let haz = pow(sin(a) * 0.5 + 0.5, 14.0);
        let c = arcoiris(floor(a) * 0.17 + A(2) * 0.07 + f32(lado) * 0.4 + floor(a / 6.283) * 0.29);
        l += c.xyz * haz * A(0) * 0.55 * (1.0 - 0.6 * clamp(v / H, 0.0, 1.0));
    }
    let d = 1.0 - 0.18 * min(A(0), 1.0);
    return pantalla(vec4<f32>(p.xyz * d, p.w), l);
}
fn E_iris_luz(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 se, 1 co, 2 g, 3 tv
    let u = ((x - CX) * A(1) + (y - CY) * A(0)) / H;
    let o = nubes(u * 1.3 + A(3) * 0.15, y / H * 0.6 + 3.0);
    let z = (u - sin(A(3) * 0.5) * 0.6) * 2.6;
    let b = exp(-z * z) * (0.5 + o);
    return pantalla(p, arcoiris(u * 0.7 + o * 0.3 + A(3) * 0.05).xyz * b * A(2) * 0.55);
}
fn E_neon(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 g, 1 osc, 2 tv; N0 uno
    let d = max(ES, 0.5) * 1.5;
    let a1 = luz(en(x - d, y - d));
    let a2 = luz(en(x, y - d));
    let a3 = luz(en(x + d, y - d));
    let b1 = luz(en(x - d, y));
    let b3 = luz(en(x + d, y));
    let c_1 = luz(en(x - d, y + d));
    let c_2 = luz(en(x, y + d));
    let c_3 = luz(en(x + d, y + d));
    let gx = a3 + 2.0 * b3 + c_3 - a1 - 2.0 * b1 - c_1;
    let gy = c_1 + 2.0 * c_2 + c_3 - a1 - 2.0 * a2 - a3;
    let b = escalon(0.12, 0.9, length(vec2<f32>(gx, gy))) * A(0);
    var c = C1();
    if (N(0) != 1) { c = arcoiris(x / W * 0.6 + y / H * 0.4 + A(2) * 0.15); }
    return pantalla(vec4<f32>(p.xyz * A(1), p.w), c.xyz * b);
}
fn E_rayos(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 g
    let q = en_aux(x * 0.25, y * 0.25);
    return pantalla(p, vec3<f32>(q.x * A(0) * 1.9, q.y * A(0) * 1.75, q.z * A(0) * 1.4));
}
fn E_relampago(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 g, 1 sx, 2 fin; N0 n
    let g = A(0);
    let base = 0.72 + 0.28 * min(g, 1.0);
    var l = g * 0.45;
    if (y < A(2) && g > 0.02) {
        var raya = A(1);
        var f = 4.0;
        var q = 1.0;
        for (var k = 0; k < 5; k++) {
            raya += (ruido(y / H * f + 30.0 * azar(N(0), 9 + k), 5.0 * f32(N(0))) - 0.5) * W * 0.22 / q;
            f = f * 2.0;
            q = q * 1.9;
        }
        let d = abs(x - raya) / (H * 0.006);
        l += g * (1.6 / (1.0 + d * d) + 0.5 * exp(-d * 0.12));
    }
    return pantalla(vec4<f32>(p.x * base, p.y * base, p.z * base * 1.04, p.w), vec3<f32>(l * 0.82, l * 0.9, l));
}
fn E_fugaces(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // data: cuántas, y por estrella ox, oy, largo, a
    var l = 0.0;
    let n = i32(data[0]);
    for (var i = 0; i < n; i++) {
        let ox = data[1 + i * 4];
        let oy = data[2 + i * 4];
        let largo = data[3 + i * 4];
        let a = data[4 + i * 4];
        let u = (x - ox) * 0.848 - (y - oy) * 0.53;
        let w2 = (x - ox) * 0.53 + (y - oy) * 0.848;
        if (u > 0.0 && u < largo) {
            let k = 1.0 - u / largo;
            let q = w2 / (2.2 * max(ES, 0.5));
            l += 1.6 * a * k * k / (1.0 + q * q);
        }
        l += a * 1.2 / (1.0 + (u * u + w2 * w2) / max(3.0 * ES * ES, 1.0));
    }
    return pantalla(p, C1().xyz * l);
}
fn E_fantasmal(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 g, 1 pul, 2 tv
    let g = A(0);
    let d = (nubes(x / H * 2.0 + A(2) * 0.3, y / H * 2.0 - A(2) * 0.5) - 0.5) * H * 0.08 * g;
    let l = luz(espejo(x + d + H * 0.03 * g, y)) * 0.5 + luz(espejo(x - d - H * 0.03 * g, y - H * 0.02 * g)) * 0.5;
    let m = mix(p, vec4<f32>(C1().xyz * l * A(1), p.w), g * 0.85);
    let u = (x - CX) / CX;
    let v = (y - CY) / CY;
    return vec4<f32>(m.xyz * (1.0 - g * 0.55 * escalon(0.25, 1.0, u * u * 0.5 + v * v * 0.5)), m.w);
}
fn E_goteo(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let g = A(0);
    let ta = H * 0.05;
    let i = floor(x / ta);
    let u = x / ta - i - 0.5;
    let r3 = azar(i32(i), 3);
    let h0 = g * H * (0.12 + 0.95 * r3 * r3);
    let base = g * H * (0.08 + 0.05 * nubes(x / H * 5.0, 2.0));
    let an = 0.22 + 0.18 * azar(i32(i), 8);
    let punta = h0 - ta * an;
    var d = abs(u);
    if (y > punta) { d = length(vec2<f32>(u, (y - punta) / ta)); }
    let a = max(1.0 - escalon(an - 0.06, an, d), 1.0 - escalon(base - 2.0, base + 2.0, y));
    if (g <= 0.0 || a <= 0.0) { return p; }
    let z = (u + an * 0.4) / 0.09;
    return mix(p, vec4<f32>(C1().xyz * (0.75 + 0.55 * exp(-z * z)), p.w), a);
}
fn E_estatica(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 g, 1 tv; N0 k
    let g = A(0);
    let fila = i32(floor(y / (H * 0.02)));
    let salto = select(0.0, (azar(fila + 3, N(0)) - 0.5) * W * 0.2 * g, azar(fila, N(0)) > 0.9);
    let q = espejo(x + salto, y);
    let e = max(ES, 0.5);
    let nie = azar(i32(x / e) + N(0) * 131, i32(y / e) + N(0) * 7);
    let m = g * (0.35 + 0.5 * escalon(0.6, 0.9, nubes(y / H * 3.0, A(1) * 2.0)));
    return mix(vec4<f32>(q.xyz, p.w), vec4<f32>(nie, nie, nie, p.w), min(m, 0.95));
}
fn E_visor(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 g; N0 rec
    let g = A(0);
    let mx = W * 0.06;
    let my = H * 0.08;
    let l = H * 0.09;
    let gr = max(H * 0.006, 1.0);
    let u = min(x, W - x);
    let v = min(y, H - y);
    let esq = (u >= mx && u < mx + l && abs(v - my) < gr) || (v >= my && v < my + l && abs(u - mx) < gr);
    let a = abs(x - CX);
    let b = abs(y - CY);
    let cruz = (a < gr * 0.5 && b < H * 0.03) || (b < gr * 0.5 && a < H * 0.03);
    let bx = x - (W - mx - H * 0.13);
    let by = y - (my + H * 0.04);
    let dentro = bx >= 0.0 && bx < H * 0.09 && by >= 0.0 && by < H * 0.04;
    let bat = (dentro && (bx < gr || bx > H * 0.09 - gr || by < gr || by > H * 0.04 - gr))
        || (dentro && bx > gr * 2.0 && bx < H * 0.06 && by > gr * 2.0 && by < H * 0.04 - gr * 2.0)
        || (bx >= H * 0.09 && bx < H * 0.097 && by > H * 0.012 && by < H * 0.028);
    let punto = 1.0 - escalon(H * 0.016, H * 0.019, length(vec2<f32>(x - (mx + H * 0.06), y - (my + H * 0.06))));
    var q = p;
    if (esq || cruz || bat) { q = mix(q, vec4<f32>(C1().xyz, max(q.w, g)), g); }
    if (N(0) == 1 && punto > 0.0) { q = mix(q, vec4<f32>(1.0, 0.12, 0.12, max(q.w, g)), punto * g); }
    return q;
}
fn E_rollo(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 an, 1 baja
    let u = min(x, W - x);
    if (u > A(0)) { return p; }
    let paso = H / 7.0;
    let v = resto(y + A(1), paso) / paso - 0.5;
    let a = abs(u / A(0) - 0.5) / 0.28;
    let b = abs(v) / 0.27;
    let d = length(vec2<f32>(max(a, 0.7) - 0.7, max(b, 0.7) - 0.7));
    let c = 0.03 + 0.9 * (1.0 - escalon(0.24, 0.3, d));
    return vec4<f32>(c, c * 0.98, c * 0.94, 1.0);
}
fn E_redondo(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 m, 1 hw, 2 hh, 3 r
    let u = abs(x - CX) - (A(1) - A(3));
    let v = abs(y - CY) - (A(2) - A(3));
    let d = length(vec2<f32>(max(u, 0.0), max(v, 0.0))) + min(max(u, v), 0.0) - A(3);
    let q = en(CX + (x - CX) * CX / max(A(1), 1.0), CY + (y - CY) * CY / max(A(2), 1.0));
    let som = 1.0 - 0.45 * (1.0 - escalon(0.0, A(0) * 0.8 + 1.0, max(d, 0.0)));
    return mix(vec4<f32>(C1().xyz * som, C1().w), q, 1.0 - escalon(-1.0 * ES, 1.0 * ES, d));
}
fn E_apertura(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 r, 1 su; N0 todo abierto
    let d = length(vec2<f32>(x - CX, y - CY));
    let a = select(escalon(A(0) - A(1), A(0) + A(1) * 0.2, d), 0.0, N(0) == 1);
    return mix(p, vec4<f32>(C1().xyz, p.w), a);
}
fn E_persianas(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 se, 1 co, 2 n, 3 a, 4 largo
    let u = (((x - CX) * A(1) + (y - CY) * A(0)) / A(4) + 0.5) * A(2);
    let d = abs(frac(u) - 0.5) * 2.0;
    let tapa = select(escalon(A(3) - 0.04, A(3) + 0.01, d), 0.0, A(3) >= 0.999);
    return vec4<f32>(p.xyz * (1.0 - tapa), p.w);
}
fn E_vhs(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 g, 1 tv; N0 k
    let g = A(0);
    let tv = A(1);
    let banda = escalon(0.93, 0.97, y / H + 0.02 * sin(tv * 1.3));
    let dx = ((ruido(y / H * 70.0, tv * 9.0) - 0.5) * 0.006 + banda * (ruido(y / H * 240.0, f32(N(0))) - 0.5) * 0.12) * W * g;
    let l = luz(espejo(x + dx, y));
    var c = vec4<f32>(0.0);
    for (var i = 0; i < 5; i++) { c += espejo(x + dx + (f32(i) - 1.0) * 0.0035 * W * max(g, 0.1), y) / 5.0; }
    let lc = luz(c);
    var q = vec3<f32>(l + (c.x - lc) * 1.15, l + (c.y - lc) * 0.95, l + (c.z - lc) * 1.1);
    let e = max(ES, 0.5);
    let nie = azar(i32(x / (2.0 * e)) + N(0) * 91, i32(y / e) + N(0) * 17) - 0.5;
    let raya = select(1.0, 0.94, i32(y / (2.0 * e)) % 2 == 0);
    q = ((q * 0.9 + 0.045) + nie * (0.10 + 0.5 * banda) * g) * (1.0 - (1.0 - raya) * min(g, 1.0));
    return vec4<f32>(q, p.w);
}
fn E_boceto(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 g, 1 k
    let d = max(ES, 0.5) * 1.2;
    let gx = luz(en(x + d, y)) - luz(en(x - d, y));
    let gy = luz(en(x, y + d)) - luz(en(x, y - d));
    let b = length(vec2<f32>(gx, gy)) * A(0) * 4.0;
    let lo = luz(p);
    let paso = 6.0 * max(ES, 0.5);
    var som = 0.0;
    if (lo < 0.55) { som += 0.28 * (1.0 - escalon(0.25, 0.6, abs(frac((x + y + 1e4) / paso) - 0.5) * 2.0)); }
    if (lo < 0.3) { som += 0.30 * (1.0 - escalon(0.25, 0.6, abs(frac((x - y + 2e4) / paso) - 0.5) * 2.0)); }
    let v = clamp(0.97 - b - som + (azar(i32(x), i32(y)) - 0.5) * 0.05, 0.0, 1.0);
    return mix(vec4<f32>(v, v, v * 0.98, p.w), vec4<f32>(p.xyz * v, p.w), A(1));
}
fn E_tramado(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    var m = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    let i = floor(x / A(0));
    let j = floor(y / A(0));
    let q = en((i + 0.5) * A(0), (j + 0.5) * A(0));
    let u = (m[(i32(j) & 3) * 4 + (i32(i) & 3)] + 0.5) / 16.0;
    let c = select(P.f[8], P.f[9], (luz(q) - 0.5) * 1.15 + 0.5 > u);
    return vec4<f32>(c.xyz, p.w);
}
fn E_codigo(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 ta, 1 g, 2 tv
    let ta = A(0);
    let g = A(1);
    let tv = A(2);
    let i = i32(floor(x / ta));
    let j = i32(floor(y / ta));
    let filas = H / ta;
    let cab = resto(tv * (4.0 + 9.0 * azar(i, 1)) + azar(i, 2) * filas * 2.0, filas * 1.7);
    let d = cab - f32(j);
    let k = i32(tv * 7.0 * (0.3 + azar(i, j)) + azar(j, i) * 50.0);
    let u = i32(frac(x / ta) * 6.0);
    let w2 = i32(frac(y / ta) * 8.0);
    let prende = u < 5 && w2 < 7 && azar(i * 37 + min(u, 4 - u) + k * 13, j * 71 + w2) > 0.52;
    var l = 0.0;
    var cabeza = 0.0;
    if (d >= 0.0 && prende) {
        l = max(1.0 - d / (filas * 0.55), 0.0);
        if (d < 1.0) { cabeza = 0.8; }
    }
    let os = 1.0 - 0.45 * min(g, 1.0);
    let z = vec4<f32>(p.x * os * 0.6, p.y * os * (0.8 + 0.2 * luz(p)), p.z * os * 0.6, p.w);
    return pantalla(z, C1().xyz * l * g + cabeza);
}
fn E_mosaico(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // 0 ta, 1 junta
    let ta = A(0);
    let i = i32(floor(x / ta));
    let j = i32(floor(y / ta));
    var d1 = 1e30;
    var d2 = 1e30;
    var o = vec2<f32>(x, y);
    for (var a = -1; a <= 1; a++) {
        for (var b = -1; b <= 1; b++) {
            let u = i + a;
            let v = j + b;
            let q = vec2<f32>((f32(u) + 0.15 + 0.7 * azar(u, v)) * ta, (f32(v) + 0.15 + 0.7 * azar(u + 91, v)) * ta);
            let d = distance(vec2<f32>(x, y), q);
            if (d < d1) {
                d2 = d1;
                d1 = d;
                o = q;
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    let q = en(o.x, o.y);
    return vec4<f32>(q.xyz * (1.0 - A(1) * (1.0 - escalon(ta * 0.03, ta * 0.12, d2 - d1))), p.w);
}
fn E_polvo2(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    // Rayas: data = cuántas y x, a, q
    var l = 0.0;
    let n = i32(data[0]);
    for (var i = 0; i < n; i++) {
        let d = abs(x - data[1 + i * 3] - sin(y / H * 6.0 + data[3 + i * 3] * 9.0) * 2.0 * ES);
        l += data[2 + i * 3] * (1.0 - escalon(0.3 * ES, 1.3 * ES, d));
    }
    return pantalla(p, vec3<f32>(l, l, l * 0.95));
}
fn E_nada(x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    return p;
}

fn bc_cierra(t: vec2<i32>, base: vec4<f32>, q0: vec4<f32>) {
    var q = q0;
    if (P.f[11].w != 1.0) { q = mix(base, q, P.f[11].w); }
    let a = clamp(q.w, 0.0, 1.0);
    textureStore(out, t, vec4<f32>(clamp(q.xyz, vec3<f32>(0.0), vec3<f32>(1.0)) * a, a));
}
fn bc_abre() {
    W = P.f[11].x;
    H = P.f[11].y;
    ES = P.f[11].z;
    CX = W / 2.0;
    CY = H / 2.0;
}
@compute @workgroup_size(16, 16)
fn bc_color(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_color(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_zoom(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_zoom(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_destello(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_destello(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_tono(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_tono(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_sacudida(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_sacudida(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_rgb(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_rgb(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_vueltas(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_vueltas(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_tv(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_tv(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_cinta(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_cinta(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_nocturna(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_nocturna(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_termica(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_termica(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_consola(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_consola(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_barras(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_barras(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_marco(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_marco(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_puntos(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_puntos(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_manga(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_manga(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_duotono(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_duotono(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_neblina(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_neblina(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_velocidad(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_velocidad(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_prisma(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_prisma(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_tunel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_tunel(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_agua(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_agua(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_halo(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_halo(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_foco(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_foco(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_vidrio(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_vidrio(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_barrido(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_barrido(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_degradado(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_degradado(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_lluvia(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_lluvia(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_radial(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_radial(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_caleidoscopio(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_caleidoscopio(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_pendulo(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_pendulo(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_rebote(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_rebote(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_remolino(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_remolino(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_onda(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_onda(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_sismo(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_sismo(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_azulejos(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_azulejos(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_espejo(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_espejo(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_giro3d(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_giro3d(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_cubo(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_cubo(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_bandera(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_bandera(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_lente(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_lente(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_gotas(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_gotas(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_linterna(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_linterna(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_discoteca(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_discoteca(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_iris_luz(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_iris_luz(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_neon(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_neon(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_rayos(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_rayos(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_relampago(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_relampago(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_fugaces(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_fugaces(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_fantasmal(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_fantasmal(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_goteo(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_goteo(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_estatica(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_estatica(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_visor(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_visor(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_rollo(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_rollo(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_redondo(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_redondo(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_apertura(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_apertura(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_persianas(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_persianas(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_vhs(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_vhs(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_boceto(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_boceto(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_tramado(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_tramado(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_codigo(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_codigo(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_mosaico(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_mosaico(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_polvo2(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_polvo2(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
@compute @workgroup_size(16, 16)
fn bc_nada(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    let p = recto(textureLoad(src, t, 0));
    bc_cierra(t, p, E_nada(f32(t.x) + 0.5, f32(t.y) + 0.5, p));
}
// ---- partículas: data = columnas, filas, inicio de cada celda (columnas·filas + 1), índices, y por partícula x, y, r, número, color
fn fig_confeti(u: f32, v: f32, i: i32, tv: f32) -> f32 {
    let g = tv * (1.0 + azar(i, 31) * 3.0) + f32(i);
    let gira = max(abs(cos(tv * (2.0 + 4.0 * azar(i, 41)) + f32(i))), 0.12);
    let a = u * cos(g) + v * sin(g);
    let b = -u * sin(g) + v * cos(g);
    return select(0.0, 1.0, abs(a) < 0.68 && abs(b) < 0.42 * gira);
}
fn fig_corazon(u: f32, v: f32, i: i32) -> f32 {
    let x = u * 1.25;
    let y = -v * 1.25 + 0.1;
    let q = x * x + y * y - 0.8;
    return 0.9 * escalon(0.06, -0.06, q * q * q - x * x * y * y * y) * (0.7 + 0.3 * azar(i, 61));
}
fn fig_luciernaga(u: f32, v: f32, i: i32, tv: f32) -> f32 {
    let late = 0.35 + 0.65 * abs(sin(tv * (1.0 + azar(i, 71) * 2.0) + f32(i)));
    let r = length(vec2<f32>(u, v));
    let k = max(1.0 - r, 0.0);
    return k * k * k * late * 0.9 + escalon(0.16, 0.08, r) * late;
}
fn fig_bokeh(u: f32, v: f32, i: i32) -> f32 {
    let d = length(vec2<f32>(u, v));
    return (1.0 - escalon(0.9, 1.0, d)) * (0.16 + 0.12 * escalon(0.7, 0.95, d)) * (0.4 + 0.6 * azar(i, 12));
}
fn fig_brillo(u: f32, v: f32, i: i32, tv: f32) -> f32 {
    let s = max(sin(tv * (1.5 + 2.0 * azar(i, 4)) + azar(i, 6) * 40.0), 0.0);
    let a = max(1.0 - abs(u), 0.0);
    let b = max(1.0 - abs(v), 0.0);
    let cruz = a * a * max(1.0 - abs(v) * 14.0, 0.0) + b * b * max(1.0 - abs(u) * 14.0, 0.0);
    let centro = max(1.0 - length(vec2<f32>(u, v)) * 5.0, 0.0);
    return min((cruz + centro) * s * s * s * 1.4, 1.0);
}
fn fig_nieve(u: f32, v: f32, i: i32) -> f32 {
    return (1.0 - escalon(0.2, 1.0, length(vec2<f32>(u, v)))) * (0.35 + 0.6 * azar(i, 12));
}
fn fig_fuego(u: f32, v: f32) -> f32 {
    return pow(max(1.0 - length(vec2<f32>(u, v)), 0.0), 1.4);
}
fn fig_globo(u: f32, v: f32) -> f32 {
    let cuerpo = 1.0 - escalon(0.93, 1.0, length(vec2<f32>(u / 0.62, (v + 0.25) / 0.72)));
    let hilo = select(0.0, 0.6, v > 0.46 && abs(u - sin(v * 9.0) * 0.05) < 0.015);
    let l = 1.0 - 0.35 * escalon(0.1, 0.9, length(vec2<f32>(u + 0.2, v + 0.5)));
    return max(cuerpo * 0.92 * l, hilo);
}
fn fig_burbuja(u: f32, v: f32) -> f32 {
    let d = length(vec2<f32>(u, v));
    let k = (d - 0.9) / 0.07;
    let b = ((u + 0.38) * (u + 0.38) + (v + 0.42) * (v + 0.42)) / 0.025;
    return min(exp(-k * k) * 0.75 + exp(-b) * 0.9 + select(0.0, 0.07, d < 0.9), 1.0);
}
fn fig_petalo(u: f32, v: f32, i: i32, tv: f32) -> f32 {
    let a = azar(i, 8) * 6.28 + tv * (1.0 + 2.0 * azar(i, 9));
    let p = u * cos(a) + v * sin(a);
    let q = (-u * sin(a) + v * cos(a)) / (0.35 + 0.4 * abs(sin(tv * 2.0 + azar(i, 6) * 9.0)));
    return (1.0 - escalon(0.62, 0.8, length(vec2<f32>(p, q)) + 0.22 * p)) * (0.75 + 0.25 * (1.0 - abs(p)));
}
fn fig_murcielago(u: f32, v: f32, i: i32, tv: f32) -> f32 {
    let ala = sin(tv * (9.0 + 4.0 * azar(i, 6)) + azar(i, 7) * 6.0) * 0.28;
    let a = abs(u);
    let arriba = -0.12 - ala * a * 1.6 - 0.16 * sin(a * 3.14);
    let abajo = arriba + 0.36 * (1.0 - a) + 0.09 * abs(sin(a * 14.0));
    let alas = a < 0.98 && v > arriba && v < abajo;
    let cuerpo = (u / 0.13) * (u / 0.13) + (v / 0.26) * (v / 0.26) < 1.0;
    let orejas = abs(a - 0.07) < 0.05 && v < -0.18 && v > -0.36 + abs(a - 0.07) * 3.0;
    return select(0.0, 0.96, alas || cuerpo || orejas);
}
fn fig_mota(u: f32, v: f32, i: i32) -> f32 {
    let w = v * (1.0 + 2.0 * azar(i, 3));
    return (1.0 - escalon(0.5, 1.0, length(vec2<f32>(u, w)))) * 0.75;
}

// u[3] = (figura, suma); A(0) = tv
@compute @workgroup_size(16, 16)
fn bc_estampa(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    W = P.f[11].x;
    H = P.f[11].y;
    let x = f32(t.x) + 0.5;
    let y = f32(t.y) + 0.5;
    var d = recto(textureLoad(src, t, 0));
    let cols = i32(data[0]);
    let filas = i32(data[1]);
    let cel = clamp(t.y * filas / dims.y, 0, filas - 1) * cols + clamp(t.x * cols / dims.x, 0, cols - 1);
    let ini = i32(data[2 + cel]);
    let fin = i32(data[3 + cel]);
    let base = 2 + cols * filas + 1;
    let parts = base + i32(data[2 + cols * filas]);
    let tv = A(0);
    for (var k = ini; k < fin; k++) {
        let o = parts + i32(data[base + k]) * 8;
        let r = data[o + 2];
        let u = (x - data[o]) / r;
        let v = (y - data[o + 1]) / r;
        if (abs(u) > 1.0 + 1.0 / r || abs(v) > 1.0 + 1.0 / r) { continue; }
        let i = i32(data[o + 3]);
        var c = 0.0;
        switch (N(0)) {
            case 0: { c = fig_confeti(u, v, i, tv); }
            case 1: { c = fig_corazon(u, v, i); }
            case 2: { c = fig_luciernaga(u, v, i, tv); }
            case 3: { c = fig_bokeh(u, v, i); }
            case 4: { c = fig_brillo(u, v, i, tv); }
            case 5: { c = fig_nieve(u, v, i); }
            case 6: { c = fig_fuego(u, v); }
            case 7: { c = fig_globo(u, v); }
            case 8: { c = fig_burbuja(u, v); }
            case 9: { c = fig_petalo(u, v, i, tv); }
            case 10: { c = fig_murcielago(u, v, i, tv); }
            default: { c = fig_mota(u, v, i); }
        }
        if (c > 0.0) {
            let col = vec4<f32>(data[o + 4], data[o + 5], data[o + 6], 1.0);
            if (N(1) == 1) {
                d = vec4<f32>(d.xyz + col.xyz * c, max(d.w, min(c, 1.0)));
            } else {
                d = sobre(d, col, c);
            }
        }
    }
    let a = clamp(d.w, 0.0, 1.0);
    textureStore(out, t, vec4<f32>(clamp(d.xyz, vec3<f32>(0.0), vec3<f32>(1.0)) * a, a));
}

// La imagen a la cuarta parte, solo lo que brilla por encima de A(0).
@compute @workgroup_size(16, 16)
fn bc_luces(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    var m = vec3<f32>(0.0);
    for (var b = 0; b < 4; b++) {
        for (var a = 0; a < 4; a++) {
            let q = px(t.x * 4 + a, t.y * 4 + b);
            m += q.xyz * q.w / 16.0;
        }
    }
    let l = max(luz(vec4<f32>(m, 1.0)), max(m.x, max(m.y, m.z)) * 0.8);
    var g = 1.0;
    if (A(0) >= 0.0) { g = escalon(A(0), min(A(0) + 0.25, 1.2), l); }
    textureStore(out, t, vec4<f32>(m * g, 1.0));
}

// Rayos desde (A1, A2) sobre las luces (a la cuarta parte).
@compute @workgroup_size(16, 16)
fn bc_rayos_paso(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    let x = (f32(t.x) + 0.5) * 4.0;
    let y = (f32(t.y) + 0.5) * 4.0;
    var q = vec3<f32>(0.0);
    let r = azar(t.x, t.y);
    var peso = 1.0;
    for (var j = 0; j < 28; j++) {
        let z = 1.0 - (f32(j) + r) / 28.0 * 0.85;
        q += en((A(1) + (x - A(1)) * z) * 0.25, (A(2) + (y - A(2)) * z) * 0.25).xyz * peso / 28.0;
        peso = peso * 0.965;
    }
    textureStore(out, t, vec4<f32>(q, 1.0));
}

// ---- entradas y pasos de la composición

// Cuadro RGBA de 8 bits; si cambia de tamaño, dos por dos muestras.
@compute @workgroup_size(16, 16)
fn bc_rgba(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    let d = vec2<i32>(textureDimensions(src));
    if (d.x == dims.x && d.y == dims.y) {
        let c = textureLoad(src, t, 0);
        textureStore(out, t, vec4<f32>(c.xyz * c.w, c.w));
        return;
    }
    var s = vec4<f32>(0.0);
    let k = vec2<f32>(d) / vec2<f32>(dims);
    for (var b = 0; b < 2; b++) {
        for (var a = 0; a < 2; a++) {
            s += en((f32(t.x) + 0.25 + 0.5 * f32(a)) * k.x, (f32(t.y) + 0.25 + 0.5 * f32(b)) * k.y) * 0.25;
        }
    }
    textureStore(out, t, vec4<f32>(s.xyz * s.w, s.w));
}

fn plano1(x: i32, y: i32, x0: i32, w: i32, y0: i32, h: i32) -> f32 {
    // U y V van a dos renglones por fila de la textura.
    let xx = clamp(x, 0, w - 1);
    let yy = clamp(y, 0, h - 1);
    let k = yy * w + xx + x0;
    let fila = i32(P.u[0].x);
    return textureLoad(src, vec2<i32>(k % fila, y0 + k / fila), 0).x;
}
fn croma(x: f32, y: f32, cual: i32) -> f32 {
    let w = i32(P.u[0].x) / 2;
    let h = i32(P.u[0].y) / 2;
    // El color va pegado a la izquierda en lo ancho y centrado en lo alto.
    let u = x * 0.5;
    let v = y * 0.5 - 0.25;
    let i = floor(u);
    let j = floor(v);
    let a = i32(i);
    let b = i32(j);
    let x0 = cual * w * h;
    let y0 = i32(P.u[0].y);
    let p = mix(plano1(a, b, x0, w, y0, h), plano1(a + 1, b, x0, w, y0, h), u - i);
    let q = mix(plano1(a, b + 1, x0, w, y0, h), plano1(a + 1, b + 1, x0, w, y0, h), u - i);
    return mix(p, q, v - j);
}
fn yuv_en(x: i32, y: i32) -> vec3<f32> {
    let w = i32(P.u[0].x);
    let h = i32(P.u[0].y);
    let cx = clamp(x, 0, w - 1);
    let cy = clamp(y, 0, h - 1);
    let l = (textureLoad(src, vec2<i32>(cx, cy), 0).x * 255.0 - 16.0) / 219.0;
    let u = (croma(f32(cx), f32(cy), 0) * 255.0 - 128.0) / 224.0;
    let v = (croma(f32(cx), f32(cy), 1) * 255.0 - 128.0) / 224.0;
    return clamp(vec3<f32>(l + 1.5748 * v, l - 0.187324 * u - 0.468124 * v, l + 1.8556 * u), vec3<f32>(0.0), vec3<f32>(1.0));
}
fn yuv_lin(x0: f32, y0: f32) -> vec3<f32> {
    let x = x0 - 0.5;
    let y = y0 - 0.5;
    let i = floor(x);
    let j = floor(y);
    let a = i32(i);
    let b = i32(j);
    return mix(mix(yuv_en(a, b), yuv_en(a + 1, b), x - i), mix(yuv_en(a, b + 1), yuv_en(a + 1, b + 1), x - i), y - j);
}
@compute @workgroup_size(16, 16)
fn bc_i420(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    let d = vec2<i32>(P.u[0].xy);
    if (d.x == dims.x && d.y == dims.y) {
        textureStore(out, t, vec4<f32>(yuv_en(t.x, t.y), 1.0));
        return;
    }
    var s = vec3<f32>(0.0);
    let k = vec2<f32>(d) / vec2<f32>(dims);
    for (var b = 0; b < 2; b++) {
        for (var a = 0; a < 2; a++) {
            s += yuv_lin((f32(t.x) + 0.25 + 0.5 * f32(a)) * k.x, (f32(t.y) + 0.25 + 0.5 * f32(b)) * k.y) * 0.25;
        }
    }
    textureStore(out, t, vec4<f32>(s, 1.0));
}

// Multiplica por A(0); con u[3].x = 1 además la deja opaca; y recorta al recuadro f[1] (por los pixeles enteros).
@compute @workgroup_size(16, 16)
fn bc_ajuste(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    var c = textureLoad(src, t, 0) * A(0);
    let r = P.f[1];
    if (f32(t.x) < r.x || f32(t.x) >= r.z || f32(t.y) < r.y || f32(t.y) >= r.w) { c = vec4<f32>(0.0); }
    if (N(0) == 1) { c.w = 1.0; }
    textureStore(out, t, c);
}

// Todo de un color (f[0], ya premultiplicado).
@compute @workgroup_size(16, 16)
fn bc_color_solido(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    textureStore(out, t, P.f[0]);
}

// Estira `src` (ya premultiplicada) por 1 / A(0), suavizando y repitiendo la orilla.
@compute @workgroup_size(16, 16)
fn bc_estirar(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    let d = vec2<i32>(textureDimensions(src)) - vec2<i32>(1);
    let q = (vec2<f32>(t) + 0.5) * A(0) - 0.5;
    let i = floor(q);
    let k = q - i;
    let a = clamp(vec2<i32>(i), vec2<i32>(0), d);
    let b = clamp(vec2<i32>(i) + vec2<i32>(1), vec2<i32>(0), d);
    let u = mix(textureLoad(src, a, 0), textureLoad(src, vec2<i32>(b.x, a.y), 0), k.x);
    let v = mix(textureLoad(src, vec2<i32>(a.x, b.y), 0), textureLoad(src, b, 0), k.x);
    textureStore(out, t, mix(u, v, k.y));
}

// ---- efectos de FilmCraft (en luz lineal, premultiplicada)

fn a_lin(v: vec3<f32>) -> vec3<f32> {
    let c = clamp(v, vec3<f32>(0.0), vec3<f32>(1.0));
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}
fn a_srgb(v: vec3<f32>) -> vec3<f32> {
    let c = max(v, vec3<f32>(0.0));
    return select(1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055, c * 12.92, c <= vec3<f32>(0.0031308));
}
fn lum3(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)); }
fn paso_s(a: f32, b: f32, x: f32) -> f32 {
    let t = clamp((x - a) / max(b - a, 1e-6), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}
fn tal(x: i32, y: i32) -> vec4<f32> {
    let d = vec2<i32>(textureDimensions(src));
    return textureLoad(src, vec2<i32>(clamp(x, 0, d.x - 1), clamp(y, 0, d.y - 1)), 0);
}
fn o_nada(x: i32, y: i32) -> vec4<f32> {
    let d = vec2<i32>(textureDimensions(src));
    if (x < 0 || y < 0 || x >= d.x || y >= d.y) { return vec4<f32>(0.0); }
    return textureLoad(src, vec2<i32>(x, y), 0);
}
// Entre pixeles; fuera de la imagen no hay nada.
fn en_t(x0: f32, y0: f32) -> vec4<f32> {
    let x = x0 - 0.5;
    let y = y0 - 0.5;
    let i = floor(x);
    let j = floor(y);
    let a = i32(i);
    let b = i32(j);
    return mix(mix(o_nada(a, b), o_nada(a + 1, b), x - i), mix(o_nada(a, b + 1), o_nada(a + 1, b + 1), x - i), y - j);
}

@compute @workgroup_size(16, 16)
fn bc_lin(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_lin(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_lin(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let a = clamp(p.w, 0.0, 1.0);
    if (a > 0.0) { return vec4<f32>(a_lin(p.xyz / a) * a, a); }
    return vec4<f32>(0.0, 0.0, 0.0, a);
}
@compute @workgroup_size(16, 16)
fn bc_srgb(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_srgb(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_srgb(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let a = clamp(p.w, 0.0, 1.0);
    if (a > 0.0) { return vec4<f32>(a_srgb(clamp(p.xyz / a, vec3<f32>(0.0), vec3<f32>(1.0))) * a, a); }
    return vec4<f32>(0.0, 0.0, 0.0, a);
}
@compute @workgroup_size(16, 16)
fn bc_fc_afin(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_afin(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_afin(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    var q = vec4<f32>(0.0);
    let n = N(0);
    for (var i = 0; i < n; i++) {
        let o = i * 6;
        q += en_t(A(o) * x + A(o + 1) * y + A(o + 2), A(o + 3) * x + A(o + 4) * y + A(o + 5));
    }
    return q / f32(n);
}
@compute @workgroup_size(16, 16)
fn bc_fc_rotar(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_rotar(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_rotar(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let u = P.f[0].xyz;
    let v = P.f[1].xyz;
    let n = P.f[2].xyz;
    let f = A(11);
    let c = vec3<f32>(0.0, 0.0, A(12));
    let d = vec3<f32>(x - CX, y - CY, f);
    let den = dot(n, d);
    if (abs(den) < 1e-12) { return vec4<f32>(0.0); }
    let k = dot(n, c) / den;
    if (k <= 0.0) { return vec4<f32>(0.0); }
    let l = d * k - c;
    return en_t(dot(l, u) + CX, dot(l, v) + CY);
}
@compute @workgroup_size(16, 16)
fn bc_fc_recorte(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_recorte(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_recorte(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let r = A(4);
    let cx = (A(0) + A(2)) / 2.0;
    let cy = (A(1) + A(3)) / 2.0;
    let qx = abs(x - cx) - ((A(2) - A(0)) / 2.0 - r);
    let qy = abs(y - cy) - ((A(3) - A(1)) / 2.0 - r);
    let d = length(vec2<f32>(max(qx, 0.0), max(qy, 0.0))) + min(max(qx, qy), 0.0) - r;
    var o = p;
    if (A(6) > 0.0) {
        let k = clamp(0.5 - d, 0.0, 1.0) - clamp(0.5 - d - A(6), 0.0, 1.0);
        if (k > 0.0) { o = vec4<f32>(P.f[2].xyz * k, k) + o * (1.0 - k); }
    }
    var a = clamp(0.5 - d, 0.0, 1.0);
    if (A(5) > 0.0) { a = clamp(-d / A(5), 0.0, 1.0); }
    return o * a;
}
@compute @workgroup_size(16, 16)
fn bc_fc_clones(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_clones(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_clones(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let g = A(2);
    let ci = min(floor(x / (A(3) + g)), A(0) - 1.0);
    let ri = min(floor(y / (A(4) + g)), A(1) - 1.0);
    let s = A(5);
    let ox = ci * (A(3) + g) + (A(3) - W * s) / 2.0;
    let oy = ri * (A(4) + g) + (A(4) - H * s) / 2.0;
    var u = (x - ox) / s;
    let v = (y - oy) / s;
    if (N(0) == 1 && i32(ci + ri) % 2 == 1) { u = W - u; }
    if (u < 0.0 || v < 0.0 || u > W || v > H) { return vec4<f32>(0.0); }
    return en_t(u, v);
}
@compute @workgroup_size(16, 16)
fn bc_fc_rgb(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_rgb(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_rgb(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    var o = vec2<f32>(A(1), A(2)) * A(0);
    if (N(0) == 1) { o = vec2<f32>(x - A(4), y - A(5)) / A(6) * A(0); }
    let r = en_t(x - o.x, y - o.y);
    let b = en_t(x + o.x, y + o.y);
    return mix(vec4<f32>(r.x, p.y, b.z, max(r.w, max(p.w, b.w))), p, A(3));
}
@compute @workgroup_size(16, 16)
fn bc_fc_fugas(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_fugas(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_fugas(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let along = ((x / W - 0.5) * A(8) + (y / H - 0.5) * A(9)) * 2.0;
    let bias = paso_s(-0.2, 1.0, -along);
    let nm = en_aux(x * A(10), y * A(11));
    let k = paso_s(0.35, 0.85, nm.x) * bias * A(3);
    let l = mix(P.f[0].xyz, P.f[1].xyz, nm.y) * k;
    let base = recto(p).xyz;
    var o = base;
    if (N(0) == 1) {
        o = base + l;
    } else if (N(0) == 2) {
        let s = mix(base, l + 0.5, min(k, 1.0));
        o = select(1.0 - 2.0 * (1.0 - min(base, vec3<f32>(1.0))) * (1.0 - min(s, vec3<f32>(1.0))), 2.0 * base * s, base < vec3<f32>(0.5));
    } else {
        o = 1.0 - (1.0 - min(base, vec3<f32>(1.0))) * (1.0 - min(l, vec3<f32>(1.0)));
    }
    let a = p.w + (1.0 - p.w) * min(k, 1.0);
    return vec4<f32>(o * a, a);
}
@compute @workgroup_size(16, 16)
fn bc_fc_claros(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_claros(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_claros(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    if (p.w <= 1e-6) { return vec4<f32>(0.0); }
    let l = lum3(a_srgb(p.xyz / p.w));
    return p * paso_s(A(0), A(0) + max(A(1), 1e-3), l);
}
@compute @workgroup_size(16, 16)
fn bc_fc_haces(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_haces(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_haces(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    var s = vec4<f32>(0.0);
    var peso = 0.0;
    for (var i = 0; i < 48; i++) {
        let t = f32(i) / 48.0 * A(2);
        let k = 1.0 - f32(i) / 48.0 * 0.6;
        s += en_t(x + (A(0) - x) * t, y + (A(1) - y) * t) * k;
        peso += k;
    }
    return s / peso;
}
@compute @workgroup_size(16, 16)
fn bc_fc_estela(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_estela(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_estela(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    return p + en_t(x - A(0), y - A(1)) * A(2);
}
@compute @workgroup_size(16, 16)
fn bc_fc_suma(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_suma(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_suma(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    return p * A(0) + textureLoad(aux, t, 0) * A(1);
}
@compute @workgroup_size(16, 16)
fn bc_fc_tinte(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_tinte(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_tinte(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    var c = p.xyz;
    if (A(4) != 1.0) {
        let l = lum3(c);
        c = max(vec3<f32>(l) + (c - l) * A(4), vec3<f32>(0.0));
    }
    if (A(3) > 0.0) { c += (lum3(c) * P.f[0].xyz * 1.5 - c) * A(3); }
    return vec4<f32>(c, p.w);
}
@compute @workgroup_size(16, 16)
fn bc_fc_luz(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_luz(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_luz(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    let l = textureLoad(aux, t, 0).xyz * P.f[0].xyz;
    if (N(0) == 1) {
        let a = min(l, vec3<f32>(1.0));
        return vec4<f32>(p.xyz + a - min(p.xyz, vec3<f32>(1.0)) * a, p.w + (1.0 - p.w) * max(a.x, max(a.y, a.z)));
    }
    return vec4<f32>(p.xyz + l, p.w + (1.0 - p.w) * clamp(max(l.x, max(l.y, l.z)), 0.0, 1.0));
}
@compute @workgroup_size(16, 16)
fn bc_fc_bordes(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_bordes(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_bordes(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    var v = array<f32, 9>();
    for (var j = 0; j < 3; j++) {
        for (var i = 0; i < 3; i++) {
            let q = tal(t.x + i - 1, t.y + j - 1);
            v[j * 3 + i] = a_srgb(vec3<f32>(clamp(lum3(q.xyz), 0.0, 1.0))).x * min(q.w, 1.0);
        }
    }
    let gx = v[2] + 2.0 * v[5] + v[8] - v[0] - 2.0 * v[3] - v[6];
    let gy = v[6] + 2.0 * v[7] + v[8] - v[0] - 2.0 * v[1] - v[2];
    return vec4<f32>(paso_s(A(0), A(0) + 0.1, length(vec2<f32>(gx, gy)) / 4.0));
}
@compute @workgroup_size(16, 16)
fn bc_fc_nivel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    bc_abre();
    textureStore(out, t, K_fc_nivel(t, f32(t.x) + 0.5, f32(t.y) + 0.5, textureLoad(src, t, 0)));
}
fn K_fc_nivel(t: vec2<i32>, x: f32, y: f32, p: vec4<f32>) -> vec4<f32> {
    return min(p * A(0), vec4<f32>(1.0));
}

// ---------------------------------------------------------------- transiciones (Beethoven Craft)
// Las mismas cuentas que `craft_trans` (legacy.rs) en luz lineal, como `composicion::transicion`:
// A en `src`, B en `aux`; P.u[0].x el tipo (0 disolvencia, 1 aditiva, 2 de cine, 3 pasar por negro, 4 por blanco),
// P.f[0].x el avance.
fn tr_lin(p: vec4<f32>) -> vec4<f32> {
    let a = clamp(p.w, 0.0, 1.0);
    if (a <= 0.0) { return vec4<f32>(0.0); }
    let s = clamp(p.xyz / a, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(vec3<f32>(srgb_to_linear(s.x), srgb_to_linear(s.y), srgb_to_linear(s.z)) * a, a);
}
fn tr_srgb(p: vec4<f32>) -> vec4<f32> {
    let a = clamp(p.w, 0.0, 1.0);
    if (a <= 0.0) { return vec4<f32>(0.0); }
    let s = clamp(p.xyz / a, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(vec3<f32>(linear_to_srgb(s.x), linear_to_srgb(s.y), linear_to_srgb(s.z)) * a, a);
}
@compute @workgroup_size(16, 16)
fn bc_transicion(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = out_dims();
    let t = vec2<i32>(gid.xy);
    if (t.x >= dims.x || t.y >= dims.y) { return; }
    let a = tr_lin(textureLoad(src, t, 0));
    let b = tr_lin(textureLoad(aux, t, 0));
    let p = clamp(P.f[0].x, 0.0, 1.0);
    var o = a * (1.0 - p) + b * p;
    switch (P.u[0].x) {
        case 1u: {
            o = min(a * min(2.0 * (1.0 - p), 1.0) + b * min(2.0 * p, 1.0), vec4<f32>(1.0));
        }
        case 2u: {
            let g = 1.0 / 2.2;
            o = pow(pow(max(a, vec4<f32>(0.0)), vec4<f32>(g)) * (1.0 - p) + pow(max(b, vec4<f32>(0.0)), vec4<f32>(g)) * p, vec4<f32>(2.2));
        }
        case 3u, 4u: {
            let fondo = select(vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(1.0), P.u[0].x == 4u);
            let s = select(b * ((p - 0.5) * 2.0), a * (1.0 - p * 2.0), p < 0.5);
            o = s + fondo * (1.0 - s.w);
        }
        default: {}
    }
    textureStore(out, t, tr_srgb(o));
}
