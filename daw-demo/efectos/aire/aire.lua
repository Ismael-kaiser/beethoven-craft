local equipo = incluir("equipo")

return {
  nombre = { es = "Excitador de agudos", en = "Treble exciter" }, tipo = "efecto", grupo = "Tono",
  controles = { { "hz", "Desde Hz", 2000, 12000, 6000 }, { "cuanto", "Cuánto", 0, 1, 0.3 } },
  presets = {
    { nombre = "Brillo suave", hz = 8000, cuanto = 0.2 },
    { nombre = "Voz con aire", hz = 6000, cuanto = 0.45 },
    { nombre = "Mucho brillo", hz = 4000, cuanto = 0.8 },
  },
  sonido = function(e, c)
    local function f(x) return x + pasaaltos(tanh(pasaaltos(x, c.hz) * 6), c.hz) * c.cuanto * 0.5 end
    return f(e.izq), f(e.der)
  end,
  interfaz = equipo.bulbos{ tinte = "#3c5560", color = "#a8ecff", metal = "#b4b7bd", empuje = function(c) return c.cuanto end, claves = { "hz", "cuanto" } },
}
