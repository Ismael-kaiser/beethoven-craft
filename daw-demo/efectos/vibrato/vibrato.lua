local equipo = incluir("equipo")

return {
  nombre = "Vibrato", tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.5, 12, 5 }, { "hondo", "Hondura", 0, 1, 0.3 } },
  presets = {
    { nombre = "De cantante", vel = 5.5, hondo = 0.2 },
    { nombre = "Disco chueco", vel = 0.8, hondo = 0.8 },
  },
  sonido = function(e, c)
    local s = 0.004 + seno(c.vel) * c.hondo * 0.003
    return retardo(e.izq, s, 0, 0.01), retardo(e.der, s, 0, 0.01)
  end,
  interfaz = equipo.pedal{ tinte = "#3d7a55", color = "#b4ffce", metal = "#c8cad0", claves = { "vel", "hondo" } },
}
