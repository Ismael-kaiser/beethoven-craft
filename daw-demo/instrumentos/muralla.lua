-- Muralla: siete sierras desafinadas y abiertas en estéreo. Para acordes grandes y melodías que llenan.

local function fila(d, m, x, y, paso, r, color, lista, metal)
  for i, p in ipairs(lista) do
    local cx = x + (i - 1) * paso
    d.perilla(cx, y, r, p[1], color, metal)
    d.grabado(cx, y + r + 11, m.idioma == "en" and p.en or p.es, 11, "#f4f1ea", "centro")
  end
end
local function marco(d, x, y, w, h, titulo, color)
  d.bandeja(x, y, w, h, nil, 7)
  d.grabado(x + 10, y + 13, titulo, 12, color)
end

return {
  nombre = { es = "Supersierra", en = "Supersaw" }, grupo = { es = "Sintetizadores propios", en = "Our synths" }, voces = 12,
  controles = {
    { "abierto", { es = "Desafinado", en = "Detune" }, 0, 1, 0.35 }, { "ancho", { es = "Ancho", en = "Width" }, 0, 1, 0.8 },
    { "sub", { es = "Grave", en = "Sub" }, 0, 1, 0.3 }, { "corte", { es = "Brillo", en = "Brightness" }, 200, 18000, 7000 },
    { "a", { es = "Ataque", en = "Attack" }, 0.001, 3, 0.02 }, { "r", { es = "Soltar", en = "Release" }, 0.01, 5, 0.5 },
  },
  presets = {
    { nombre = { es = "Melodía de festival", en = "Festival lead" }, abierto = 0.4, corte = 12000, a = 0.002, r = 0.2, sub = 0.15 },
    { nombre = { es = "Acordes enormes", en = "Huge chords" }, abierto = 0.55, ancho = 1, corte = 5000, a = 0.08, r = 1.2 },
    { nombre = { es = "Niebla", en = "Fog" }, abierto = 0.7, corte = 1400, a = 1.5, r = 3, sub = 0.5 },
  },
  sonido = function(n, c)
    local lados = { 0, 0 }
    local desvios = { -1, -0.63, -0.31, 0, 0.29, 0.65, 0.98 }
    for i, v in ipairs(desvios) do
      local o = sierra(n.hz * (1 + v * c.abierto * 0.025), (i * 0.3819) % 1)
      local p = (i % 2 == 0 and 1 or -1) * (i == 4 and 0 or 1)
      lados[1] = lados[1] + o * (1 - p * c.ancho)
      lados[2] = lados[2] + o * (1 + p * c.ancho)
    end
    local grave = seno(n.hz * 0.5) * c.sub * 3
    local e = adsr(c.a, 0.5, 0.85, c.r) * n.fuerza * 0.065
    return pasabajos(lados[1] + grave, c.corte, 0.05) * e, pasabajos(lados[2] + grave, c.corte, 0.05) * e
  end,
  interfaz = {
    ancho = 460, alto = 250,
    dibujar = function(d, c, m)
      d.acero(0, 0, 460, 250, "#6a3f5a", 10)
      local en = m.idioma == "en"
      d.grabado(30, 24, m.titulo, 22, "#ff9ad0")
      d.hueco(14, 42, 432, 96, 6)
      local desvios = { -1, -0.63, -0.31, 0, 0.29, 0.65, 0.98 }
      for i, v in ipairs(desvios) do
        local x = 230 + v * (20 + c.abierto * 190)
        local alto = 22 + 48 * (1 - math.abs(v) * 0.5)
        d.degradado(x - 4, 132 - alto, 8, alto, "#ffd0ea", "#a8306c", 3)
        d.led(x, 124 - alto, 3, "#ff9ad0", 1)
      end
      d.bandeja(14, 150, 432, 88, nil, 8)
      fila(d, m, 56, 184, 70, 24, "#ff9ad0", { { "abierto", es = "Desafinado", en = "Detune" }, { "ancho", es = "Ancho", en = "Width" }, { "sub", es = "Grave", en = "Sub" }, { "corte", es = "Brillo", en = "Bright" }, { "a", es = "Ataque", en = "Attack" }, { "r", es = "Soltar", en = "Release" } }, "#6a3a58")
    end,
  },
}
