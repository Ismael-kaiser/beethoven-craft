local equipo = incluir("equipo")

return {
  nombre = { es = "Ensanchador estéreo", en = "Stereo enhancer" }, tipo = "efecto", grupo = "Espacio",
  controles = { { "ancho", "Ancho", 0, 2, 1.3 }, { "mono", "Mono hasta Hz", 20, 400, 120 },
    { "desfase", "Retrasa un lado (s)", -0.02, 0.02, 0 }, { "lado", "Lado", -1, 1, 0 }, { "volumen", "Volumen", 0, 2, 1 }, { "volteado", "Voltea la derecha", 0, 1, 0 } },
  presets = {
    { nombre = "Un poco más ancho", ancho = 1.2, mono = 100 },
    { nombre = "Muy ancho", ancho = 1.8, mono = 180 },
    { nombre = "Casi mono", ancho = 0.3, mono = 20 },
  },
  sonido = function(e, c)
    local m, l = (e.izq + e.der) * 0.5, (e.izq - e.der) * 0.5
    l = pasaaltos(l, c.mono, 0) * c.ancho
    local a = si(mayor(c.desfase, 0.00002), retardo(m + l, maximo(c.desfase, 0), 0, 0.03), m + l)
    local b = si(mayor(0 - c.desfase, 0.00002), retardo(m - l, maximo(0 - c.desfase, 0), 0, 0.03), m - l)
    b = b * (1 - 2 * mayor(c.volteado, 0.5))
    return a * minimo(1, 1 - c.lado) * c.volumen, b * minimo(1, 1 + c.lado) * c.volumen
  end,
  interfaz = equipo.rack{ tinte = "#54506c", color = "#c4b8ff", metal = "#8d8f96", grafica = false, aguja = function(c, m) return equipo.vu(m.nivel), "VU" end, claves = { "ancho", "mono", "desfase", "lado", "volumen", "volteado" } },
}
