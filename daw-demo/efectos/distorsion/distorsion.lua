local equipo = incluir("equipo")

-- Distorsión directa: preamplifica, recorta de dos maneras y mezcla.
return {
  nombre = { es = "Distorsión", en = "Distortion" }, tipo = "efecto", grupo = "Carácter",
  controles = {
    { "pre", { es = "Preamplifica dB", en = "Pre amp dB" }, 0, 40, 14 }, { "umbral", { es = "Umbral", en = "Threshold" }, 0.05, 1, 0.5 },
    { "duro", { es = "Suave o dura", en = "Soft or hard" }, 0, 1, 0 }, { "mezcla", "Mezcla", 0, 1, 1 }, { "sale", "Salida dB", -24, 12, -4 },
  },
  presets = { { nombre = { es = "Crujido", en = "Crunch" }, pre = 10, umbral = 0.6 }, { nombre = { es = "Fuzz", en = "Fuzz" }, pre = 34, umbral = 0.25, duro = 1, sale = -2 }, { nombre = { es = "Paralela", en = "Parallel" }, pre = 24, umbral = 0.3, mezcla = 0.35 } },
  sonido = function(e, c)
    local function f(x)
      local y = x * db(c.pre) / c.umbral
      local blanda = curva(y, function(v) return math.tanh(v) end, "suave")
      local dura = curva(y, function(v) return math.max(-1, math.min(1, v)) end, "suave")
      return mezclar(x, mezclar(blanda, dura, c.duro) * c.umbral, c.mezcla) * db(c.sale)
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = equipo.pedal{ tinte = "#b8862a", color = "#ffe08a", metal = "#2c2d30", claves = { "pre", "umbral", "duro", "mezcla", "sale" }, letras = "#1c1408" },
}
