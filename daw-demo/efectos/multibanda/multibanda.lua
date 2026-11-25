return {
  nombre = "Multibanda", tipo = "efecto", grupo = "Dinámica",
  controles = {
    { "corte1", "Corte bajo", 60, 500, 160 }, { "corte2", "Corte alto", 1000, 8000, 3000 },
    { "bajo", "Graves", 0, 1, 0.5 }, { "medio", "Medios", 0, 1, 0.4 }, { "alto", "Agudos", 0, 1, 0.4 },
    { "sube", "Salida dB", -12, 18, 3 },
  },
  sonido = function(e, c)
    local function aplasta(x, cuanto)
      local nivel = seguidor(abs(x), 0.008, 0.16)
      return x * maximo(nivel / db(-30 + (1 - cuanto) * 30), 1) ^ (0 - 0.7 * cuanto)
    end
    local function f(x)
      local b = pasabajos(x, c.corte1, 0)
      local a = pasaaltos(x, c.corte2, 0)
      return (aplasta(b, c.bajo) + aplasta(x - b - a, c.medio) + aplasta(a, c.alto)) * db(c.sube)
    end
    return f(e.izq), f(e.der)
  end,
}
