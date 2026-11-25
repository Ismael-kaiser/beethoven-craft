local equipo = incluir("equipo")

return {
  nombre = { es = "Eco de cinta", en = "Tape delay" }, tipo = "efecto", grupo = "Espacio",
  controles = { { "seg", "Tiempo", 0.02, 1.5, 0.45 }, { "repite", "Repite", 0, 0.95, 0.4 }, { "mezcla", "Mezcla", 0, 1, 0.3 }, { "seco", "Seco", 0, 1, 1 },
    { "corte", "Quita graves", 20, 1000, 20 }, { "tope", "Quita agudos", 1000, 18000, 18000 },
    { "negras", "Negras (0: libre)", 0, 4, 0 }, { "lado", "Entre lados", 0.25, 1, 0.75 }, { "flota", "Oscilación", 0, 1, 0 }, { "empuje", "Saturación", 0, 1, 0 } },
  presets = {
    { nombre = "Eco corto de voz", seg = 0.12, repite = 0.2, mezcla = 0.22, tope = 6000 },
    { nombre = "Eco de cinta", seg = 0.38, repite = 0.55, mezcla = 0.3, corte = 250, tope = 3500 },
    { nombre = "Eco largo", seg = 0.75, repite = 0.6, mezcla = 0.35 },
  },
  sonido = function(e, c)
    local seg = si(mayor(c.negras, 0.05), minimo(negras(c.negras), 1.5), c.seg) * (1 + seno(0.8) * c.flota * 0.004)
    local a = retardo(e.izq, seg, c.repite, 1.7)
    local b = retardo(e.der, seg * c.lado, c.repite, 1.7)
    local function limpia(x)
      x = pasaaltos(pasabajos(x, c.tope, 0), c.corte, 0)
      return mezclar(x, curva(x * 3, function(v) return math.tanh(v) end, "suave") * 0.5, c.empuje)
    end
    return e.izq * c.seco + limpia(a) * c.mezcla, e.der * c.seco + limpia(b) * c.mezcla
  end,
  interfaz = equipo.cinta{ tinte = "#5a6152", color = "#c8f08a", metal = "#8d8f96", por = 5, vueltas = function(c) return 0.15 + 0.12 / c.seg end, claves = { "seg", "negras", "repite", "lado", "flota", "empuje", "corte", "tope", "mezcla", "seco" } },
}
