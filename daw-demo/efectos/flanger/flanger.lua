local equipo = incluir("equipo")

return {
  nombre = "Flanger", tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.02, 4, 0.25 }, { "hondo", "Hondura", 0, 1, 0.7 }, { "repite", "Repite", -0.95, 0.95, 0.5 }, { "mezcla", "Mezcla", 0, 1, 0.5 },
    { "base", "Retardo", 0.0005, 0.008, 0.0035 }, { "fase", "Entre lados", 0, 0.5, 0.25 }, { "opaco", "Quita agudos", 1000, 18000, 18000 }, { "volteado", "Voltea la fase", 0, 1, 0 } },
  presets = {
    { nombre = "Suave", vel = 0.15, hondo = 0.5, repite = 0.3, mezcla = 0.4 },
    { nombre = "Avión", vel = 0.1, hondo = 1, repite = 0.85, mezcla = 0.8 },
    { nombre = "Metálico", vel = 1.5, hondo = 0.4, repite = -0.8 },
  },
  sonido = function(e, c)
    local function f(x, fase)
      local s = maximo(c.base + seno(c.vel, fase) * c.hondo * 0.003, 0.0001)
      local y = pasabajos(retardo(x, s, c.repite, 0.012), c.opaco, 0) * (1 - 2 * mayor(c.volteado, 0.5))
      return x + y * c.mezcla
    end
    return f(e.izq, 0), f(e.der, c.fase)
  end,
  interfaz = equipo.pedal{ tinte = "#7a3f74", color = "#ffb0f4", metal = "#c8cad0", por = 4, claves = { "vel", "hondo", "repite", "base", "fase", "opaco", "mezcla", "volteado" } },
}
