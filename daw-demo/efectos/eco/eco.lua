return {
  nombre = "Eco", tipo = "efecto", grupo = "Espacio",
  controles = { { "seg", "Tiempo", 0.02, 1.5, 0.45 }, { "repite", "Repite", 0, 0.95, 0.4 }, { "mezcla", "Mezcla", 0, 1, 0.3 }, { "seco", "Seco", 0, 1, 1 } },
  sonido = function(e, c)
    local a = retardo(e.izq, c.seg, c.repite, 1.6)
    local b = retardo(e.der, c.seg * 0.75, c.repite, 1.6)
    return e.izq * c.seco + a * c.mezcla, e.der * c.seco + b * c.mezcla
  end,
}
