local equipo = incluir("equipo")

return {
  nombre = { es = "Phaser", en = "Phaser" }, tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.02, 6, 0.4 }, { "centro", "Centro Hz", 200, 4000, 900 }, { "hondo", "Hondura", 0, 1, 0.8 }, { "mezcla", "Mezcla", 0, 1, 0.5 },
    { "etapas", "Etapas", 4, 12, 6 }, { "fase", "Entre lados", 0, 0.5, 0.2 }, { "sale", "Salida dB", -18, 18, 0 } },
  presets = {
    { nombre = "Lento", vel = 0.15, hondo = 0.7 },
    { nombre = "Setentero", vel = 0.6, centro = 1100, hondo = 1, mezcla = 0.8 },
    { nombre = "Rápido", vel = 3.5, hondo = 0.5 },
  },
  sonido = function(e, c)
    local function f(x, fase)
      local hz = c.centro * 2 ^ (seno(c.vel, fase) * c.hondo * 2)
      local y = x
      local ultima = 0
      for i = 1, 12 do
        y = pasatodo(y, hz * (0.6 + 0.2 * i))
        if i >= 4 then ultima = si(mayor(c.etapas, i - 0.5), y, ultima) end
      end
      y = ultima
      return mezclar(x, (x + y) * 0.5, c.mezcla * 2 - c.mezcla * c.mezcla) * db(c.sale)
    end
    return f(e.izq, 0), f(e.der, c.fase)
  end,
  interfaz = equipo.pedal{ tinte = "#b5672a", color = "#ffd9a0", metal = "#3c3e42", por = 4, claves = { "vel", "centro", "hondo", "etapas", "fase", "mezcla", "sale" } },
}
