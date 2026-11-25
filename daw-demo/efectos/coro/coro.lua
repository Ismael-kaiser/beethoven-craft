return {
  nombre = "Coro", tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.05, 5, 0.6 }, { "hondo", "Hondura", 0, 1, 0.5 }, { "mezcla", "Mezcla", 0, 1, 0.5 } },
  sonido = function(e, c)
    local function f(x, fase) return x + retardo(x, 0.012 + seno(c.vel, fase) * c.hondo * 0.006, 0, 0.03) * c.mezcla end
    return f(e.izq, 0), f(e.der, 0.25)
  end,
}
