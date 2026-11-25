local equipo = incluir("equipo")

return {
  nombre = "Trémolo", tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.2, 20, 5 }, { "hondo", "Hondura", 0, 1, 0.6 }, { "gira", "Entre lados", 0, 0.5, 0 } },
  presets = {
    { nombre = "Suave", vel = 3.5, hondo = 0.35 },
    { nombre = "De amplificador", vel = 6, hondo = 0.75 },
    { nombre = "De lado a lado", vel = 4, hondo = 0.9, gira = 0.5 },
  },
  sonido = function(e, c)
    local function g(fase) return 1 - c.hondo * (0.5 + 0.5 * seno(c.vel, fase)) end
    return e.izq * g(0), e.der * g(c.gira)
  end,
  interfaz = equipo.pedal{ tinte = "#8a7a30", color = "#fff0a0", metal = "#3c3e42", claves = { "vel", "hondo", "gira" } },
}
