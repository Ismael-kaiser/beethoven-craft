local equipo = incluir("equipo")

return {
  nombre = { es = "Saturación de cinta", en = "Tape saturation" }, tipo = "efecto", grupo = "Carácter",
  controles = { { "fuego", "Saturación", 0.5, 6, 1.6 }, { "flota", "Oscilación", 0, 1, 0.15 }, { "agudos", "Agudos", 2000, 18000, 11000 }, { "ruido", "Soplido", 0, 1, 0 } },
  presets = {
    { nombre = "Calor suave", fuego = 1.3, flota = 0.08, agudos = 14000 },
    { nombre = "Casete viejo", fuego = 2.2, flota = 0.5, agudos = 7000, ruido = 0.35 },
    { nombre = "Cinta apretada", fuego = 4, flota = 0.1, agudos = 10000 },
  },
  sonido = function(e, c)
    local s = 0.004 + (seno(0.7) * 0.6 + seno(5.3, 0.3) * 0.15) * c.flota * 0.0016
    local function f(x)
      x = retardo(x, s, 0, 0.01)
      return pasabajos(tanh(x * c.fuego + 0.05) / c.fuego, c.agudos, 0) + pasabanda(ruido(), 4000, 0) * c.ruido * 0.01
    end
    return pasaaltos(f(e.izq), 20), pasaaltos(f(e.der), 20)
  end,
  interfaz = equipo.cinta{ tinte = "#6c5a44", color = "#ffcf8a", metal = "#c9b58a", claves = { "fuego", "flota", "agudos", "ruido" } },
}
