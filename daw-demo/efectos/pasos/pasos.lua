-- Pasos: dieciséis casillas al ritmo de la canción que dejan pasar el sonido o lo bajan.
local pasos = {}
for i = 1, 16 do pasos[i] = { "p" .. i, { es = "Paso " .. i, en = "Step " .. i }, 0, 1, i % 2 == 1 and 1 or 0 } end

local function patron(nombre, dibujo)
  local p = { nombre = nombre }
  for i = 1, 16 do p["p" .. i] = dibujo:sub(i, i) == "x" and 1 or 0 end
  return p
end

local controles = {
  { "div", { es = "Pasos por negra", en = "Steps per beat" }, 1, 8, 4 },
  { "hondo", { es = "Hondura", en = "Depth" }, 0, 1, 1 },
  { "suave", { es = "Suavidad", en = "Smoothness" }, 0.001, 0.08, 0.004 },
}
for _, p in ipairs(pasos) do controles[#controles + 1] = p end

return {
  nombre = { es = "Compuerta rítmica", en = "Rhythm gate" }, tipo = "efecto", grupo = { es = "Movimiento", en = "Movement" },
  controles = controles,
  presets = {
    patron({ es = "Corcheas", en = "Eighths" }, "xx..xx..xx..xx.."),
    patron({ es = "Galope", en = "Gallop" }, "x.xxx.xxx.xxx.xx"),
    patron({ es = "Tresillo", en = "Tresillo" }, "x..x..x.x..x..x."),
  },
  sonido = function(e, c)
    local s = piso(resto(posicion() * piso(c.div + 0.5), 16))
    local abierto = s * 0
    for i = 1, 16 do abierto = abierto + mayor(s, i - 1.5) * mayor(i - 0.5, s) * c["p" .. i] end
    local g = 1 - c.hondo * (1 - abierto)
    g = seguidor(g, c.suave, c.suave)
    return e.izq * g, e.der * g
  end,
  interfaz = {
    ancho = 480, alto = 190,
    dibujar = function(d, c, m)
      d.acero(0, 0, 480, 190, "#245a58", 10)
      d.grabado(30, 23, m.titulo, 22, "#5ee8d4")
      d.bandeja(9, 39, 462, 98, "#12302f", 7)
      for i = 1, 16 do
        local x = 14 + (i - 1) * 28.4 + math.floor((i - 1) / 4) * 1.5
        d.boton(x, 44, 24, 74, "", "p" .. i, (i - 1) % 8 < 4 and "#5ee8d4" or "#ffd95e")
        d.grabado(x + 12, 128, tostring(i), 10, "#cfeee9", "centro")
      end
      local nombres = m.idioma == "en" and { "Per beat", "Depth", "Smooth" } or { "Por negra", "Hondura", "Suavidad" }
      for i, k in ipairs({ "div", "hondo", "suave" }) do
        local x = 96 + (i - 1) * 122
        d.perilla(x, 161, 19, k, "#5ee8d4")
        d.grabado(x + 26, 160, nombres[i], 12, "#e6faf6")
      end
    end,
  },
}
