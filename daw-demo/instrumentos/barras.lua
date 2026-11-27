-- Barras: nueve armónicos que se suman, como las barras de un órgano.

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
  nombre = { es = "Órgano de barras", en = "Drawbar organ" }, grupo = { es = "Sintetizadores propios", en = "Our synths" }, voces = 16,
  controles = {
    { "h1", { es = "Barra 1", en = "Bar 1" }, 0, 1, 0.8 },
    { "h2", { es = "Barra 2", en = "Bar 2" }, 0, 1, 1 },
    { "h3", { es = "Barra 3", en = "Bar 3" }, 0, 1, 0.6 },
    { "h4", { es = "Barra 4", en = "Bar 4" }, 0, 1, 0.4 },
    { "h5", { es = "Barra 5", en = "Bar 5" }, 0, 1, 0 },
    { "h6", { es = "Barra 6", en = "Bar 6" }, 0, 1, 0.2 },
    { "h7", { es = "Barra 7", en = "Bar 7" }, 0, 1, 0 },
    { "h8", { es = "Barra 8", en = "Bar 8" }, 0, 1, 0 },
    { "h9", { es = "Barra 9", en = "Bar 9" }, 0, 1, 0.15 },
    { "golpecito", { es = "Percusión", en = "Percussion" }, 0, 1, 0.2 }, { "tiembla", { es = "Tiembla", en = "Vibrato" }, 0, 1, 0.25 },
  },
  presets = {
    { nombre = { es = "Iglesia", en = "Church" }, h1 = 1.0, h2 = 0.5, h3 = 1.0, h4 = 0.75, h5 = 0.375, h6 = 0.5, h7 = 0.25, h8 = 0.25, h9 = 0.375, golpecito = 0, tiembla = 0.1 },
    { nombre = { es = "Jazz", en = "Jazz" }, h1 = 1.0, h2 = 1.0, h3 = 1.0, h4 = 0.0, h5 = 0.0, h6 = 0.0, h7 = 0.0, h8 = 0.0, h9 = 0.0, golpecito = 0.6 },
    { nombre = { es = "Gospel", en = "Gospel" }, h1 = 1.0, h2 = 1.0, h3 = 1.0, h4 = 1.0, h5 = 0.75, h6 = 0.625, h7 = 0.625, h8 = 0.75, h9 = 1.0, tiembla = 0.5 },
  },
  sonido = function(n, c)
    local razones = { 0.5, 1.5, 1, 2, 3, 4, 5, 6, 8 }
    local vib = seno(6.3) * c.tiembla * 0.004
    local x = golpe(0.18) * seno(n.hz * 3) * c.golpecito
    for i, r in ipairs(razones) do x = x + seno(n.hz * r, vib * r) * c["h" .. i] end
    return x * adsr(0.006, 0.05, 1, 0.04) * n.fuerza * 0.09
  end,
  interfaz = {
    ancho = 470, alto = 280,
    dibujar = function(d, c, m)
      d.acero(0, 0, 470, 280, "#5c3d28", 10)
      local en = m.idioma == "en"
      d.grabado(30, 24, m.titulo, 22, "#ffe1a8")
      d.onda(250, 10, 190, 28, "#ffe1a8")
      d.bandeja(14, 42, 356, 224, "#2a1c12", 8)
      local pies = { "16", "5⅓", "8", "4", "2⅔", "2", "1⅗", "1⅓", "1" }
      for i = 1, 9 do
        local x = 26 + (i - 1) * 38
        local col = (i < 3) and "#a8402c" or ((i == 5 or i == 7 or i == 8) and "#2a2a2e" or "#e8e2d4")
        d.tirador(x, 54, 28, 180, "h" .. i, col)
        d.grabado(x + 14, 250, pies[i], 12, "#ffe1a8", "centro")
      end
      fila(d, m, 420, 84, 70, 24, "#ffe1a8", { { "golpecito", es = "Percusión", en = "Perc" } }, "#a8823c")
      fila(d, m, 420, 168, 70, 24, "#ffe1a8", { { "tiembla", es = "Tiembla", en = "Vibrato" } }, "#a8823c")
      d.led(420, 238, 8, "#ff9a52", math.min(1, m.nivel * 3))
    end,
  },
}
