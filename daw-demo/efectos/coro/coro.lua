local equipo = incluir("equipo")

return {
  nombre = "Coro", tipo = "efecto", grupo = "Movimiento",
  controles = { { "vel", "Velocidad", 0.05, 5, 0.6 }, { "hondo", "Hondura", 0, 1, 0.5 }, { "mezcla", "Mezcla", 0, 1, 0.5 },
    { "base", "Retardo", 0.008, 0.03, 0.012 }, { "voz2", "Segunda voz", 0, 1, 0 }, { "voz3", "Tercera voz", 0, 1, 0 }, { "cruce", "Desde Hz", 20, 1000, 20 } },
  presets = {
    { nombre = "Coro suave", vel = 0.4, hondo = 0.35, mezcla = 0.35 },
    { nombre = "Guitarra ancha", vel = 0.9, hondo = 0.6, mezcla = 0.6 },
    { nombre = "Mareado", vel = 2.8, hondo = 0.9, mezcla = 0.8 },
  },
  sonido = function(e, c)
    local function f(x, fase)
      -- Los graves se quedan quietos.
      local y = si(mayor(c.cruce, 21), pasaaltos(x, c.cruce, 0), x)
      local function voz(k, vel, f2) return retardo(y, c.base * k + seno(c.vel * vel, fase + f2) * c.hondo * 0.006, 0, 0.05) end
      return x + voz(1, 1, 0) * c.mezcla + voz(1.31, 0.73, 0.33) * c.voz2 * c.mezcla + voz(1.57, 1.37, 0.66) * c.voz3 * c.mezcla
    end
    return f(e.izq, 0), f(e.der, 0.25)
  end,
  interfaz = equipo.pedal{ tinte = "#2f6f86", color = "#9feaff", metal = "#e0e2e6", por = 4, claves = { "vel", "hondo", "mezcla", "base", "voz2", "voz3", "cruce" } },
}
