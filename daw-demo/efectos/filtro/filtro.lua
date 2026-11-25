return {
  nombre = "Filtro", tipo = "efecto", grupo = "Tono",
  controles = { { "alto", "Quita graves", 15, 2000, 15 }, { "bajo", "Quita agudos", 200, 18000, 18000 }, { "filo", "Filo", 0, 0.9, 0 } },
  sonido = function(e, c)
    local function f(x) return pasabajos(pasaaltos(x, c.alto, c.filo), c.bajo, c.filo) end
    return f(e.izq), f(e.der)
  end,
}
