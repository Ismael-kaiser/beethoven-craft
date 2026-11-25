-- Siete bandas. En el Mixer se mueve con la curva.
return {
  nombre = "Ecualizador", tipo = "efecto", grupo = "Tono",
  controles = {
    { "h1", "Graves Hz", 20, 500, 80 }, { "g1", "Graves dB", -18, 18, 0 }, { "q1", "Graves ancho", 0.3, 4, 0.7 },
    { "h2", "2 Hz", 40, 2000, 200 }, { "g2", "2 dB", -18, 18, 0 }, { "q2", "2 ancho", 0.3, 12, 1 },
    { "h3", "3 Hz", 100, 5000, 600 }, { "g3", "3 dB", -18, 18, 0 }, { "q3", "3 ancho", 0.3, 12, 1 },
    { "h4", "4 Hz", 200, 10000, 1800 }, { "g4", "4 dB", -18, 18, 0 }, { "q4", "4 ancho", 0.3, 12, 1 },
    { "h5", "5 Hz", 500, 16000, 4500 }, { "g5", "5 dB", -18, 18, 0 }, { "q5", "5 ancho", 0.3, 12, 1 },
    { "h6", "Agudos Hz", 1000, 18000, 9000 }, { "g6", "Agudos dB", -18, 18, 0 }, { "q6", "Agudos ancho", 0.3, 4, 0.7 },
    { "sale", "Salida dB", -18, 18, 0 },
  },
  sonido = function(e, c)
    local function f(x)
      x = graves(x, c.h1, c.g1, c.q1)
      x = campana(x, c.h2, c.g2, c.q2)
      x = campana(x, c.h3, c.g3, c.q3)
      x = campana(x, c.h4, c.g4, c.q4)
      x = campana(x, c.h5, c.g5, c.q5)
      return agudos(x, c.h6, c.g6, c.q6) * db(c.sale)
    end
    return f(e.izq), f(e.der)
  end,
}
