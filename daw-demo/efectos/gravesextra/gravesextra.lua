return {
  nombre = "Graves extra", tipo = "efecto", grupo = "Tono",
  controles = { { "hz", "Hz", 30, 150, 70 }, { "sube", "Sube dB", 0, 15, 5 }, { "calor", "Calor", 0, 1, 0.3 } },
  sonido = function(e, c)
    local function f(x)
      local b = pasabajos(x, c.hz * 1.6, 0)
      return graves(x, c.hz, c.sube, 0.8) + (tanh(b * 4) - b) * c.calor * 0.3
    end
    return f(e.izq), f(e.der)
  end,
}
