return {
  nombre = "Reverb", tipo = "efecto", grupo = "Espacio",
  controles = { { "tam", "Tamaño", 0, 1, 0.6 }, { "brillo", "Brillo", 0, 1, 0.5 }, { "mezcla", "Mezcla", 0, 1, 0.3 }, { "seco", "Seco", 0, 1, 1 } },
  sonido = function(e, c)
    local a = sala(e.izq + e.der * 0.3, c.tam, c.brillo)
    local b = sala(retardo(e.der, 0.007) + e.izq * 0.3, c.tam, c.brillo)
    return e.izq * c.seco + a * c.mezcla, e.der * c.seco + b * c.mezcla
  end,
}
