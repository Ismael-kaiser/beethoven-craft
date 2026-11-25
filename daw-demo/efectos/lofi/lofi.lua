return {
  nombre = "Lo-fi", tipo = "efecto", grupo = "Carácter",
  controles = { { "viejo", "Viejo", 0, 1, 0.5 }, { "flota", "Flota", 0, 1, 0.4 }, { "polvo", "Polvo", 0, 1, 0.15 } },
  sonido = function(e, c)
    local s = 0.005 + seno(0.45) * c.flota * 0.003
    local polvo = pasaaltos(maximo(abs(ruido()) - 0.998, 0) * 300, 800) * c.polvo
    local function f(x)
      x = retardo(x, s, 0, 0.012)
      x = retener(x, 44100 - c.viejo * 32000)
      x = pasabajos(pasaaltos(x, 30 + c.viejo * 200), 16000 - c.viejo * 12500, 0.1)
      return tanh(x * 1.4) * 0.8 + polvo
    end
    return f(e.izq), f(e.der)
  end,
}
