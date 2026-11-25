local equipo = incluir("equipo")

return {
  nombre = { es = "Paneo automático", en = "Auto pan" }, tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.05, 10, 0.8 }, { "hondo", "Hondura", 0, 1, 0.8 } },
  presets = {
    { nombre = "Paseo lento", vel = 0.25, hondo = 0.6 },
    { nombre = "De lado a lado", vel = 1, hondo = 1 },
    { nombre = "Rápido", vel = 6, hondo = 0.7 },
  },
  sonido = function(e, c)
    local p = seno(c.vel) * c.hondo
    return e.izq * minimo(1, 1 - p), e.der * minimo(1, 1 + p)
  end,
  interfaz = equipo.pedal{ tinte = "#3a5f8f", color = "#a8d0ff", metal = "#c8cad0", claves = { "vel", "hondo" } },
}
