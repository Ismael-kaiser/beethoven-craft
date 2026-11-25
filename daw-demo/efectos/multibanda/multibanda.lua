local equipo = incluir("equipo")

return {
  nombre = { es = "Compresor multibanda", en = "Multiband compressor" }, tipo = "efecto", grupo = "Dinámica",
  controles = {
    { "corte1", "Corte bajo", 60, 500, 160 }, { "corte2", "Corte alto", 1000, 8000, 3000 },
    { "bajo", "Comprime graves", 0, 1, 0.5 }, { "medio", "Comprime medios", 0, 1, 0.4 }, { "alto", "Comprime agudos", 0, 1, 0.4 },
    { "sube", "Salida dB", -12, 18, 3 },
  },
  presets = {
    { nombre = "Master parejo", bajo = 0.4, medio = 0.3, alto = 0.3, sube = 2 },
    { nombre = "Todo al frente", bajo = 0.8, medio = 0.7, alto = 0.7, sube = 6 },
    { nombre = "Controlar graves", bajo = 0.75, medio = 0.1, alto = 0.1, sube = 1 },
  },
  sonido = function(e, c)
    local function aplasta(x, cuanto, nombre)
      local nivel = seguidor(abs(x), 0.008, 0.16)
      return x * mide(nombre, maximo(nivel / db(-30 + (1 - cuanto) * 30), 1) ^ (0 - 0.7 * cuanto))
    end
    local function f(x)
      local b = pasabajos(x, c.corte1, 0)
      local a = pasaaltos(x, c.corte2, 0)
      return (aplasta(b, c.bajo, "bajo") + aplasta(x - b - a, c.medio, "medio") + aplasta(a, c.alto, "alto")) * db(c.sube)
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = equipo.rack{ tinte = "#2c3a52", color = "#7fd0ff", metal = "#8d8f96", grafica = false,
    arriba = function(d, c, m, x, y, w, h)
      local en = m.idioma == "en"
      for i, k in ipairs({ "bajo", "medio", "alto" }) do
        d.aguja(x + (i - 1) * 168, y + 4, 148, 98, equipo.baja(m, k), ({ en and "LOW" or "GRAVES", en and "MID" or "MEDIOS", en and "HIGH" or "AGUDOS" })[i])
      end
    end, claves = { "corte1", "corte2", "bajo", "medio", "alto", "sube" } },
}
