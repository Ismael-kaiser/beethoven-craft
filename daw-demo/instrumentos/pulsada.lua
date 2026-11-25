-- Pulsada: una cuerda calculada. Se le da un golpecito de ruido y suena sola, como al pulsarla.

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
  nombre = { es = "Cuerda pulsada", en = "Plucked string" }, grupo = { es = "Sintetizadores propios", en = "Our synths" }, voces = 12,
  controles = {
    { "largo", { es = "Largo", en = "Length" }, 0.1, 8, 2.5 }, { "brillo", { es = "Brillo", en = "Brightness" }, 0, 1, 0.55 },
    { "pua", { es = "Púa", en = "Pick" }, 300, 12000, 4000 }, { "doble", { es = "Doble cuerda", en = "Double string" }, 0, 1, 0.25 },
    { "caja", { es = "Caja", en = "Body" }, 0, 12, 4 },
  },
  presets = {
    { nombre = { es = "Nailon", en = "Nylon" }, largo = 1.8, brillo = 0.35, pua = 2200, caja = 6 },
    { nombre = { es = "Acero de doce", en = "Twelve string" }, largo = 3.5, brillo = 0.8, pua = 8000, doble = 0.9 },
    { nombre = { es = "Arpa", en = "Harp" }, largo = 5, brillo = 0.5, pua = 3500, doble = 0.1, caja = 2 },
    { nombre = { es = "Apagada", en = "Muted" }, largo = 0.25, brillo = 0.4, pua = 5000 },
  },
  sonido = function(n, c)
    local toque = pasabajos(ruido(), c.pua, 0) * golpe(0.008) * n.fuerza
    local a = cuerda(toque, n.hz, c.largo, c.brillo)
    local b = cuerda(toque, n.hz * 1.004, c.largo * 0.9, c.brillo) * c.doble
    local function caja(x) return campana(x, 180, c.caja, 1.4) end
    local e = adsr(0.001, 0.1, 1, 0.12) * 0.45
    return caja(a + b * 0.4) * e, caja(a * 0.8 + b) * e
  end,
  interfaz = {
    ancho = 430, alto = 240,
    dibujar = function(d, c, m)
      d.acero(0, 0, 430, 240, "#7a5a30", 10)
      local en = m.idioma == "en"
      d.grabado(30, 24, m.titulo, 22, "#ffd98a")
      d.hueco(14, 42, 402, 100, 8)
      for i = 1, 6 do
        local v = {}
        for j = 0, 40 do
          local vibra = math.sin(j / 40 * math.pi) * (m.onda[j * 5 + i * 3] or 0) * 14
          v[#v + 1] = 20 + j * 9.75
          v[#v + 1] = 50 + i * 14 + vibra
        end
        d.linea(v, "#ffe9bb", 0.6 + (7 - i) * 0.3)
      end
      d.bandeja(14, 154, 402, 78, nil, 8)
      fila(d, m, 55, 184, 80, 22, "#ffd98a", { { "largo", es = "Largo", en = "Length" }, { "brillo", es = "Brillo", en = "Bright" }, { "pua", es = "Púa", en = "Pick" }, { "doble", es = "Doble", en = "Double" }, { "caja", es = "Caja", en = "Body" } }, "#a8823c")
    end,
  },
}
