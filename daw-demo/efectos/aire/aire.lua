return {
  nombre = "Aire", tipo = "efecto", grupo = "Tono",
  controles = { { "hz", "Desde Hz", 2000, 12000, 6000 }, { "cuanto", "Cuánto", 0, 1, 0.3 } },
  sonido = function(e, c)
    local function f(x) return x + pasaaltos(tanh(pasaaltos(x, c.hz) * 6), c.hz) * c.cuanto * 0.5 end
    return f(e.izq), f(e.der)
  end,
}
