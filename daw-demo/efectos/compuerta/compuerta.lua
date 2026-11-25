return {
  nombre = "Compuerta", tipo = "efecto", grupo = "Dinámica",
  controles = { { "umbral", "Umbral dB", -80, 0, -45 }, { "ataque", "Ataque", 0.0005, 0.1, 0.002 }, { "soltar", "Soltar", 0.02, 1, 0.15 }, { "fondo", "Fondo", 0, 1, 0 } },
  sonido = function(e, c)
    local nivel = seguidor(maximo(abs(e.izq), abs(e.der)), 0.001, 0.03)
    local g = seguidor(minimo(1, (nivel / db(c.umbral)) ^ 4), c.ataque, c.soltar)
    g = mezclar(g, 1, c.fondo)
    return e.izq * g, e.der * g
  end,
}
