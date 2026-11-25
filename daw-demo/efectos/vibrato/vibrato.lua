return {
  nombre = "Vibrato", tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.5, 12, 5 }, { "hondo", "Hondura", 0, 1, 0.3 } },
  sonido = function(e, c)
    local s = 0.004 + seno(c.vel) * c.hondo * 0.003
    return retardo(e.izq, s, 0, 0.01), retardo(e.der, s, 0, 0.01)
  end,
}
