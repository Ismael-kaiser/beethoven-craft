local equipo = incluir("equipo")

return {
  nombre = { es = "Compuerta de ruido", en = "Noise gate" }, tipo = "efecto", grupo = "Dinámica",
  controles = { { "umbral", "Umbral dB", -80, 0, -45 }, { "ataque", "Ataque", 0.0005, 0.1, 0.002 }, { "soltar", "Soltar", 0.02, 1, 0.15 }, { "fondo", "Fondo", 0, 1, 0 } },
  presets = {
    { nombre = "Quitar soplido", umbral = -55, soltar = 0.25 },
    { nombre = "Batería seca", umbral = -30, ataque = 0.0005, soltar = 0.06 },
    { nombre = "Suave", umbral = -45, soltar = 0.4, fondo = 0.25 },
  },
  sonido = function(e, c)
    local nivel = seguidor(maximo(abs(e.izq), abs(e.der)), 0.001, 0.03)
    local g = seguidor(minimo(1, (nivel / db(c.umbral)) ^ 4), c.ataque, c.soltar)
    g = mide("reduccion", mezclar(g, 1, c.fondo))
    return e.izq * g, e.der * g
  end,
  interfaz = equipo.rack{ tinte = "#33553f", color = "#9df0a8", metal = "#505258", aguja = function(c, m) return equipo.baja(m), m.idioma == "en" and "REDUCTION" or "REDUCCIÓN" end, claves = { "umbral", "ataque", "soltar", "fondo" } },
}
