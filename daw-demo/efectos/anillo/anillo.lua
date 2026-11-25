local equipo = incluir("equipo")

return {
  nombre = { es = "Modulador de anillo", en = "Ring modulator" }, tipo = "efecto", grupo = "Carácter",
  controles = { { "hz", "Hz", 1, 4000, 220 }, { "mezcla", "Mezcla", 0, 1, 0.5 } },
  presets = {
    { nombre = "Temblor grave", hz = 30, mezcla = 0.5 },
    { nombre = "Robot", hz = 220, mezcla = 0.9 },
    { nombre = "Campana rara", hz = 1300, mezcla = 0.6 },
  },
  sonido = function(e, c)
    local m = seno(c.hz)
    return mezclar(e.izq, e.izq * m, c.mezcla), mezclar(e.der, e.der * m, c.mezcla)
  end,
  interfaz = equipo.pedal{ tinte = "#8f3a3a", color = "#ffc0a8", metal = "#3c3e42", claves = { "hz", "mezcla" } },
}
