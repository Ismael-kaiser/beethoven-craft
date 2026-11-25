local equipo = incluir("equipo")

return {
  nombre = { es = "Realce de graves", en = "Bass enhancer" }, tipo = "efecto", grupo = "Tono",
  controles = { { "hz", "Hz", 30, 150, 70 }, { "sube", "Sube dB", 0, 15, 5 }, { "calor", "Armónicos", 0, 1, 0.3 } },
  presets = {
    { nombre = "Bombo gordo", hz = 60, sube = 6, calor = 0.2 },
    { nombre = "Bajo que se oye", hz = 90, sube = 4, calor = 0.7 },
  },
  sonido = function(e, c)
    local function f(x)
      local b = pasabajos(x, c.hz * 1.6, 0)
      return graves(x, c.hz, c.sube, 0.8) + (tanh(b * 4) - b) * c.calor * 0.3
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = equipo.bulbos{ tinte = "#46355c", color = "#d0a8ff", metal = "#8d8f96", empuje = function(c) return c.calor end, claves = { "hz", "sube", "calor" } },
}
