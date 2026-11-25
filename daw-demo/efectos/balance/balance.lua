local equipo = incluir("equipo")

-- Volumen y lado, con su medidor.
return {
  nombre = { es = "Balance y volumen", en = "Balance and volume" }, tipo = "efecto", grupo = { es = "Estéreo", en = "Stereo" },
  controles = { { "lado", "Lado", -1, 1, 0 }, { "volumen", "Volumen", 0, 2, 1 } },
  presets = { { nombre = { es = "A la izquierda", en = "To the left" }, lado = -0.6 }, { nombre = { es = "A la derecha", en = "To the right" }, lado = 0.6 }, { nombre = { es = "Más bajo", en = "Softer" }, volumen = 0.5 } },
  sonido = function(e, c)
    return e.izq * minimo(1, 1 - c.lado) * c.volumen, e.der * minimo(1, 1 + c.lado) * c.volumen
  end,
  interfaz = equipo.rack{ tinte = "#4a4e57", color = "#ffd95e", metal = "#b4b7bd", grafica = false, aguja = function(c, m) return equipo.vu(m.nivel), "VU" end, claves = { "lado", "volumen" } },
}
