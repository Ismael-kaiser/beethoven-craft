local equipo = incluir("equipo")

return {
  nombre = { es = "Lo-fi (disco viejo)", en = "Lo-fi (old record)" }, tipo = "efecto", grupo = "Carácter",
  controles = { { "viejo", "Desgaste", 0, 1, 0.5 }, { "flota", "Oscilación", 0, 1, 0.4 }, { "polvo", "Polvo", 0, 1, 0.15 } },
  presets = {
    { nombre = "Un poco viejo", viejo = 0.3, flota = 0.2, polvo = 0.08 },
    { nombre = "Disco rayado", viejo = 0.7, flota = 0.5, polvo = 0.6 },
    { nombre = "Muy roto", viejo = 1, flota = 0.8, polvo = 0.3 },
  },
  sonido = function(e, c)
    local s = 0.005 + seno(0.45) * c.flota * 0.003
    -- El polvo es del disco: solo se oye mientras suena algo.
    local gira = minimo(seguidor(maximo(abs(e.izq), abs(e.der)), 0.005, 0.25) * 40, 1)
    local polvo = pasaaltos(maximo(abs(ruido()) - 0.998, 0) * 300, 800) * c.polvo * gira
    local function f(x)
      x = retardo(x, s, 0, 0.012)
      x = retener(x, 44100 - c.viejo * 32000)
      x = pasabajos(pasaaltos(x, 30 + c.viejo * 200), 16000 - c.viejo * 12500, 0.1)
      return tanh(x * 1.4) * 0.8 + polvo
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = equipo.cinta{ tinte = "#5a3f34", color = "#ffb48a", metal = "#a8823c", vueltas = function() return 0.55 end, claves = { "viejo", "flota", "polvo" } },
}
