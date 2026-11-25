return {
  nombre = "Fuego", tipo = "efecto", grupo = "Carácter",
  controles = { { "fuego", "Fuego", 1, 20, 3 }, { "mezcla", "Mezcla", 0, 1, 0.6 }, { "nivel", "Nivel", 0, 1.5, 0.8 } },
  sonido = function(e, c)
    local function f(x) return mezclar(x, tanh(x * c.fuego), c.mezcla) * c.nivel end
    return f(e.izq), f(e.der)
  end,
}
