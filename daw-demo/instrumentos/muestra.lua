-- Toca el WAV del canal; la nota Do5 (60) lo toca tal cual.
return {
  nombre = "Muestra", grupo = "Muestras", voces = 8,
  controles = {
    { "tono", "Tono", -24, 24, 0 },
    { "corte", "Corte", 100, 18000, 18000 },
    { "baja", "Soltar", 0.005, 3, 0.05 },
  },
  sonido = function(n, c)
    local v = n.hz / 261.6256 * 2 ^ (c.tono / 12)
    local e = adsr(0.001, 0.1, 1, c.baja) * n.fuerza
    return pasabajos(muestra(v, "izq"), c.corte, 0) * e, pasabajos(muestra(v, "der"), c.corte, 0) * e
  end,
}
