return {
  nombre = "Auto pan", tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.05, 10, 0.8 }, { "hondo", "Hondura", 0, 1, 0.8 } },
  sonido = function(e, c)
    local p = seno(c.vel) * c.hondo
    return e.izq * minimo(1, 1 - p), e.der * minimo(1, 1 + p)
  end,
}
