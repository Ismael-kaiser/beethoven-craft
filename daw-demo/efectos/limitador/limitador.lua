-- Mira 3 ms adelante para bajar antes del golpe.
local equipo = incluir("equipo")

return {
  nombre = "Limitador", tipo = "efecto", grupo = "Dinámica",
  controles = { { "entra", "Entrada dB", 0, 24, 0 }, { "techo", "Techo dB", -12, 0, -0.5 }, { "soltar", "Soltar", 0.01, 1, 0.12 }, { "suave", "Saturación suave", 0, 1, 0 } },
  presets = {
    { nombre = "Master suave", entra = 3, techo = -1, soltar = 0.2 },
    { nombre = "Fuerte", entra = 9, techo = -0.8, soltar = 0.08 },
    { nombre = "Solo protege", entra = 0, techo = -0.3 },
  },
  sonido = function(e, c)
    local techo = db(c.techo)
    local function entra(x)
      x = x * db(c.entra)
      return mezclar(x, curva(x, function(v) return math.tanh(v) end, "suave"), c.suave)
    end
    local a, b = entra(e.izq), entra(e.der)
    local g = mide("reduccion", techo / maximo(seguidor(maximo(abs(a), abs(b)), 0.0006, c.soltar), techo))
    local function corta(x) return maximo(minimo(x, techo), 0 - techo) end
    return corta(retardo(a, 0.003) * g), corta(retardo(b, 0.003) * g)
  end,
  interfaz = equipo.rack{ tinte = "#6a2e2e", color = "#ffb070", metal = "#8d8f96", aguja = function(c, m) return equipo.baja(m), m.idioma == "en" and "REDUCTION" or "REDUCCIÓN" end, claves = { "entra", "suave", "techo", "soltar" } },
}
