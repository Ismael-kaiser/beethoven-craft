return {
  nombre = "Anillo", tipo = "efecto", grupo = "Carácter",
  controles = { { "hz", "Hz", 1, 4000, 220 }, { "mezcla", "Mezcla", 0, 1, 0.5 } },
  sonido = function(e, c)
    local m = seno(c.hz)
    return mezclar(e.izq, e.izq * m, c.mezcla), mezclar(e.der, e.der * m, c.mezcla)
  end,
}
