return {
  nombre = "Cinta", tipo = "efecto", grupo = "Carácter",
  controles = { { "fuego", "Nivel", 0.5, 6, 1.6 }, { "flota", "Flota", 0, 1, 0.15 }, { "agudos", "Agudos", 2000, 18000, 11000 }, { "ruido", "Soplido", 0, 1, 0 } },
  sonido = function(e, c)
    local s = 0.004 + (seno(0.7) * 0.6 + seno(5.3, 0.3) * 0.15) * c.flota * 0.0016
    local function f(x)
      x = retardo(x, s, 0, 0.01)
      return pasabajos(tanh(x * c.fuego + 0.05) / c.fuego, c.agudos, 0) + pasabanda(ruido(), 4000, 0) * c.ruido * 0.01
    end
    return pasaaltos(f(e.izq), 20), pasaaltos(f(e.der), 20)
  end,
}
