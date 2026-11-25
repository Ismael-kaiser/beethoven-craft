local equipo = incluir("equipo")

return {
  nombre = { es = "Quita eses", en = "De-esser" }, tipo = "efecto", grupo = "Dinámica",
  controles = { { "hz", "Desde Hz", 3000, 10000, 5500 }, { "umbral", "Umbral dB", -60, 0, -32 }, { "cuanto", "Cuánto", 0, 1, 0.7 } },
  presets = {
    { nombre = "Eses suaves", hz = 6000, umbral = -30, cuanto = 0.45 },
    { nombre = "Eses fuertes", hz = 5200, umbral = -38, cuanto = 0.85 },
  },
  sonido = function(e, c)
    local function f(x)
      local ese = pasaaltos(x, c.hz, 0.1)
      local g = mide("reduccion", maximo(seguidor(abs(ese), 0.001, 0.06) / db(c.umbral), 1) ^ (0 - 0.8 * c.cuanto))
      return x - ese * (1 - g)
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = equipo.rack{ que = "agudos", tinte = "#5c4a74", color = "#e0b8ff", metal = "#8d8f96", aguja = function(c, m) return equipo.baja(m), m.idioma == "en" and "REDUCTION" or "REDUCCIÓN" end, claves = { "hz", "umbral", "cuanto" } },
}
