-- Retumbo: bombos y bajos largos como de caja de ritmos, con el tono cayendo al golpear.

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
  nombre = { es = "Bombo y 808", en = "Kick and 808" }, grupo = { es = "Sintetizadores propios", en = "Our synths" }, voces = 4,
  controles = {
    { "caida", { es = "Cae", en = "Drop" }, 0, 36, 18 }, { "rapido", { es = "Rápido", en = "Speed" }, 0.01, 0.5, 0.06 },
    { "largo", { es = "Largo", en = "Length" }, 0.05, 6, 0.9 }, { "golpe", { es = "Golpe", en = "Click" }, 0, 1, 0.3 },
    { "fuego", { es = "Fuego", en = "Drive" }, 0, 1, 0.2 },
  },
  presets = {
    { nombre = { es = "Bombo seco", en = "Dry kick" }, caida = 30, rapido = 0.04, largo = 0.25, golpe = 0.5 },
    { nombre = { es = "Bajo largo", en = "Long bass" }, caida = 8, rapido = 0.08, largo = 3.5, golpe = 0.1, fuego = 0.25 },
    { nombre = { es = "Distorsionado", en = "Distorted" }, caida = 20, largo = 1.4, golpe = 0.4, fuego = 0.85 },
  },
  sonido = function(n, c)
    local hz = n.hz * 2 ^ (golpe(c.rapido) * c.caida / 12)
    local x = seno(hz) * golpe(c.largo) + pasaaltos(ruido(), 1800) * golpe(0.012) * c.golpe
    x = curva(x * (1 + c.fuego * 8), function(v) return math.tanh(v) end, "suave")
    return x * adsr(0.001, 0.1, 1, 0.08) * n.fuerza * (0.7 - c.fuego * 0.25)
  end,
  interfaz = {
    ancho = 430, alto = 260,
    dibujar = function(d, c, m)
      d.acero(0, 0, 430, 260, "#6e3a2c", 10)
      local en = m.idioma == "en"
      d.grabado(30, 24, m.titulo, 22, "#ffae6a")
      d.onda(220, 10, 180, 28, "#ffae6a")
      d.hueco(14, 42, 270, 110, 6)
      -- Cómo se apaga y cómo cae el tono, según las perillas.
      local nivel, tono = {}, {}
      for i = 0, 100 do
        local t = i / 100 * 1.5
        nivel[#nivel + 1] = 18 + i * 2.62
        nivel[#nivel + 1] = 146 - math.exp(-6.9 * t / c.largo) * 96
        tono[#tono + 1] = 18 + i * 2.62
        tono[#tono + 1] = 146 - math.exp(-6.9 * t / c.rapido) * c.caida / 36 * 96
      end
      d.linea(nivel, "#ffae6a", 2)
      d.linea(tono, "#7fd0ff", 1.4)
      d.aguja(300, 52, 112, 80, math.min(1, m.nivel * 1.6), en and "LEVEL" or "NIVEL")
      d.bandeja(14, 162, 402, 86, nil, 8)
      fila(d, m, 55, 194, 80, 23, "#ffae6a", { { "caida", es = "Cae", en = "Drop" }, { "rapido", es = "Rápido", en = "Speed" }, { "largo", es = "Largo", en = "Length" }, { "golpe", es = "Golpe", en = "Click" }, { "fuego", es = "Fuego", en = "Drive" } }, "#8a5a3a")
    end,
  },
}
