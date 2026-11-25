return {
  nombre = "Flanger", tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.02, 4, 0.25 }, { "hondo", "Hondura", 0, 1, 0.7 }, { "repite", "Repite", -0.95, 0.95, 0.5 }, { "mezcla", "Mezcla", 0, 1, 0.5 } },
  sonido = function(e, c)
    local function f(x, fase) return x + retardo(x, 0.0035 + seno(c.vel, fase) * c.hondo * 0.003, c.repite, 0.012) * c.mezcla end
    return f(e.izq, 0), f(e.der, 0.25)
  end,
}
