return {
  nombre = "Trémolo", tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.2, 20, 5 }, { "hondo", "Hondura", 0, 1, 0.6 } },
  sonido = function(e, c)
    local g = 1 - c.hondo * (0.5 + 0.5 * seno(c.vel))
    return e.izq * g, e.der * g
  end,
}
