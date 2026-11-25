return {
  nombre = "Quita eses", tipo = "efecto", grupo = "Dinámica",
  controles = { { "hz", "Desde Hz", 3000, 10000, 5500 }, { "umbral", "Umbral dB", -60, 0, -32 }, { "cuanto", "Cuánto", 0, 1, 0.7 } },
  sonido = function(e, c)
    local function f(x)
      local ese = pasaaltos(x, c.hz, 0.1)
      local g = maximo(seguidor(abs(ese), 0.001, 0.06) / db(c.umbral), 1) ^ (0 - 0.8 * c.cuanto)
      return x - ese * (1 - g)
    end
    return f(e.izq), f(e.der)
  end,
}
