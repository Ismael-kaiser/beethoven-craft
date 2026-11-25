local equipo = incluir("equipo")

return {
  nombre = { es = "Saturador de bulbos", en = "Tube saturator" }, tipo = "efecto", grupo = "Carácter",
  controles = { { "fuego", "Saturación", 1, 20, 3 }, { "mezcla", "Mezcla", 0, 1, 0.6 }, { "nivel", "Nivel", 0, 1.5, 0.8 },
    { "lado", "Disparejo", 0, 0.8, 0 }, { "tope", "Quita agudos", 500, 18000, 18000 } },
  presets = {
    { nombre = "Calor", fuego = 2, mezcla = 0.4, nivel = 0.9 },
    { nombre = "Distorsión", fuego = 12, mezcla = 1, nivel = 0.5, tope = 7000 },
    { nombre = "Bulbos", fuego = 4, mezcla = 0.8, lado = 0.4, tope = 9000 },
  },
  sonido = function(e, c)
    local function f(x) return mezclar(x, pasabajos(tanh(x * c.fuego + c.lado) - tanh(c.lado), c.tope, 0), c.mezcla) * c.nivel end
    return f(e.izq), f(e.der)
  end,
  interfaz = equipo.bulbos{ tinte = "#5c3a2a", color = "#ffb060", metal = "#c9b58a", empuje = function(c) return c.fuego / 20 end, claves = { "fuego", "lado", "tope", "mezcla", "nivel" } },
}
