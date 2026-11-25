-- La voz metida en una caja: rebotes de sus paredes, la tapa, el material, distorsión y una reverberación chica.
-- Trae su propia interfaz: la caja crece, se abre y vibra con el sonido.
local function mezcla_color(a, b, k)
  local function c(i) return math.floor(a[i] + (b[i] - a[i]) * k + 0.5) end
  return string.format("#%02x%02x%02x", c(1), c(2), c(3))
end

return {
  nombre = { es = "Voz en caja", en = "Voice in a box" }, tipo = "efecto", grupo = "Voz",
  controles = {
    { "tam", "Tamaño", 0.05, 1.5, 0.3 },
    { "tapa", "Tapa", 0, 1, 0.6 },
    { "duro", "Material", 0, 1, 0.45 },
    { "cuerpo", "Resonancia", 0, 1, 0.55 },
    { "fuego", "Distorsión", 0, 1, 0.25 },
    { "roto", "Bocina rota", 0, 1, 0 },
    { "sala", "Eco de la caja", 0, 1, 0.2 },
    { "mezcla", "Mezcla", 0, 1, 0.9 },
    { "sale", "Salida dB", -18, 18, 0 },
  },

  sonido = function(e, c)
    -- Ida y vuelta entre paredes: 343 m/s.
    local ida = c.tam * 2 / 343
    local rebota = 0.35 + c.cuerpo * 0.5
    local agudo = 1200 + c.duro * 8500
    local function f(x, k)
      -- El aire de la caja: cuatro pares de paredes que no coinciden.
      local r = retardo(x, ida * k, rebota, 0.02)
              + retardo(x, ida * 1.27 * k, rebota * 0.95, 0.025)
              + retardo(x, ida * 1.61 * k, rebota * 0.9, 0.03)
              + retardo(x, ida * 2.13 * k, rebota * 0.85, 0.04)
      r = pasabajos(r, agudo, 0.1) * 0.3
      -- Su nota: la de una caja de ese largo.
      local dentro = campana(x * 0.7 + r * (0.4 + c.cuerpo), minimo(172 / c.tam, 3000), 4 + c.cuerpo * 8, 1.4)
      -- La tapa cerrada quita los agudos y los graves de afuera.
      dentro = pasabajos(dentro, 14000 - c.tapa * 12800, 0.15)
      dentro = pasaaltos(dentro, 40 + c.tapa * 240, 0)
      -- Distorsión de la voz: suave, y luego la bocina rota.
      local gana = 1 + c.fuego * c.fuego * 28
      local d = tanh(dentro * gana) / (1 + c.fuego * 3.2)
      d = mezclar(d, pasabanda(escalon(retener(d, 22000 - c.roto * 17000), 64 - c.roto * 56), 1700, 0.2) * 1.8, c.roto)
      d = d + sala(d, 0.12 + c.tam * 0.25, c.duro) * c.sala * 0.6
      return mezclar(x, d, c.mezcla) * db(c.sale)
    end
    return f(e.izq, 1), f(e.der, 1.04)
  end,

  interfaz = {
    ancho = 560, alto = 360,
    dibujar = function(d, c, m)
      local es = m.idioma == "es"
      d.degradado(0, 0, 560, 360, "#26211b", "#0f0d0b", 10)
      d.metal(8, 8, 544, 40, 7)
      d.tornillo(22, 28) d.tornillo(538, 28)
      d.texto(280, 28, es and "VOZ EN CAJA" or "VOICE IN A BOX", 22, "#ffd95e", "centro")

      -- La pantalla con la caja.
      d.hueco(16, 58, 300, 222, 8)
      local tam = 36 + (c.tam / 1.5) ^ 0.6 * 60
      local tiembla = m.tocando and math.sin(m.tiempo * 70) * m.nivel * 5 * (0.3 + c.cuerpo) or 0
      local x, y = 166 + tiembla, 186 + tam * 0.35
      local madera = { 150, 100, 58 }
      local metal = { 150, 160, 175 }
      local function tono(k) return mezcla_color({ 0, 0, 0 }, { madera[1] + (metal[1] - madera[1]) * c.duro, madera[2] + (metal[2] - madera[2]) * c.duro, madera[3] + (metal[3] - madera[3]) * c.duro }, k) end
      local px, py = tam * 0.55, tam * 0.42
      -- Sombra, cara de atrás, fondo, y la luz que sale.
      d.poligono({ x - tam - 8, y + tam * 0.75 + 6, x + tam + px + 10, y + tam * 0.75 - py + 10, x + tam + 8, y + tam * 0.75 + 12, x - tam, y + tam * 0.75 + 12 }, "#00000066")
      d.poligono({ x - tam + px, y - tam * 0.75 - py, x + tam + px, y - tam * 0.75 - py, x + tam, y - tam * 0.75, x - tam, y - tam * 0.75 }, tono(0.18))
      local brillo = math.min(1, m.nivel * 1.6)
      local abierta = 1 - c.tapa
      if abierta > 0.02 then
        -- El sonido que se escapa.
        for i = 1, 4 do
          local k = (m.tiempo * 1.3 + i * 0.25) % 1
          local a = math.floor(brillo * abierta * (1 - k) * 160)
          if a > 3 then d.aro(x + px * 0.5, y - tam * 0.75 - py * 0.5 - k * 46, 10 + k * 44, string.format("#ffd95e%02x", a), 2) end
        end
      end
      d.poligono({ x + tam, y - tam * 0.75, x + tam + px, y - tam * 0.75 - py, x + tam + px, y + tam * 0.75 - py, x + tam, y + tam * 0.75 }, tono(0.5))
      d.degradado(x - tam, y - tam * 0.75, tam * 2, tam * 1.5, tono(1), tono(0.62))
      d.borde(x - tam, y - tam * 0.75, tam * 2, tam * 1.5, "#000000aa", 1.5)
      -- La bocina de enfrente, que se enciende con la distorsión.
      local rb = tam * 0.5
      d.circulo(x, y, rb + 4, "#0b0b0c")
      d.circulo(x, y, rb, mezcla_color({ 34, 34, 38 }, { 255, 70, 30 }, math.min(1, c.fuego * (0.35 + brillo))))
      for i = 1, 3 do d.aro(x, y, rb * i / 3.4 + brillo * 2, "#00000088", 1.5) end
      d.circulo(x, y, rb * 0.22 + brillo * 3, "#1a1a1d")
      if c.roto > 0.02 then
        d.linea({ x - rb * 0.8, y - rb * 0.5, x - rb * 0.25, y - rb * 0.05, x - rb * 0.4, y + rb * 0.2, x + rb * 0.2 * c.roto, y + rb * 0.75 }, "#000000", 1 + c.roto * 3)
      end
      -- La tapa, con bisagra atrás.
      local ang = abierta * 2.0
      local tx, ty = px * math.cos(ang), -py * math.cos(ang) + tam * 1.1 * math.sin(ang)
      local ax, ay = x + px, y - tam * 0.75 - py
      d.poligono({ ax - tam, ay, ax + tam, ay, ax + tam - tx, ay - ty, ax - tam - tx, ay - ty }, tono(0.82))
      d.linea({ ax - tam, ay, ax + tam, ay, ax + tam - tx, ay - ty, ax - tam - tx, ay - ty, ax - tam, ay }, "#000000aa", 1.5)
      d.vidrio(16, 58, 300, 222, "#ffd95e", 8)
      d.texto(28, 268, string.format("%.0f cm", c.tam * 100), 14, "#ffd95e")
      d.texto(304, 268, c.duro > 0.66 and (es and "metal" or "metal") or c.duro > 0.33 and (es and "madera" or "wood") or (es and "cartón" or "cardboard"), 14, "#ffd95e", "derecha")
      -- Arrastrar sobre la caja cambia su tamaño y la tapa.
      d.pad(16, 58, 300, 222, "tam", "tapa")

      -- Las perillas.
      local p = {
        { "cuerpo", es and "Resonancia" or "Resonance" }, { "duro", "Material" }, { "sala", es and "Eco" or "Echo" },
        { "fuego", es and "Distorsión" or "Distortion", "#ff6a3c" }, { "roto", es and "Bocina rota" or "Broken speaker", "#ff6a3c" }, { "tapa", es and "Tapa" or "Lid" },
      }
      for i, q in ipairs(p) do
        local col, fila = (i - 1) % 3, math.floor((i - 1) / 3)
        local qx, qy = 366 + col * 74, 112 + fila * 108
        d.metal(qx - 34, qy - 54, 68, 100, 6)
        d.texto(qx, qy - 41, q[2], 12, "#cfc9bd", "centro")
        d.perilla(qx, qy + 4, 24, q[1], q[3])
        d.texto(qx, qy + 36, string.format("%.0f %%", c[q[1]] * 100), 12, "#ffffff", "centro")
      end
      d.metal(16, 290, 528, 60, 7)
      d.texto(34, 306, es and "Mezcla" or "Mix", 13, "#cfc9bd")
      d.tirador(34, 318, 210, 22, "mezcla")
      d.texto(300, 306, es and "Salida" or "Output", 13, "#cfc9bd")
      d.tirador(300, 318, 160, 22, "sale")
      d.texto(528, 329, string.format("%+.1f dB", c.sale), 14, "#ffd95e", "derecha")
    end,
  },
}
