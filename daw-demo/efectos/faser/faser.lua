return {
  nombre = "Faser", tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.02, 6, 0.4 }, { "centro", "Centro Hz", 200, 4000, 900 }, { "hondo", "Hondura", 0, 1, 0.8 }, { "mezcla", "Mezcla", 0, 1, 0.5 } },
  sonido = function(e, c)
    local function f(x, fase)
      local hz = c.centro * 2 ^ (seno(c.vel, fase) * c.hondo * 2)
      local y = x
      for i = 1, 6 do y = pasatodo(y, hz * (0.6 + 0.2 * i)) end
      return mezclar(x, (x + y) * 0.5, c.mezcla * 2 - c.mezcla * c.mezcla)
    end
    return f(e.izq, 0), f(e.der, 0.2)
  end,
}
