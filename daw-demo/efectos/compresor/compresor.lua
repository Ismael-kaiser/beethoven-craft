local equipo = incluir("equipo")

return {
  nombre = "Compresor", tipo = "efecto", grupo = "Dinámica",
  controles = {
    { "umbral", "Umbral dB", -60, 0, -18 }, { "razon", "Razón", 1, 20, 4 },
    { "ataque", "Ataque", 0.0005, 0.2, 0.01 }, { "soltar", "Soltar", 0.02, 1.5, 0.15 },
    { "sube", "Salida dB", -12, 24, 0 }, { "mezcla", "Mezcla", 0, 1, 1 },
    { "oye", "Oye desde Hz", 20, 500, 20 },
  },
  presets = {
    { nombre = "Voz pareja", umbral = -22, razon = 3, ataque = 0.008, soltar = 0.12, sube = 4, oye = 90 },
    { nombre = "Batería con pegada", umbral = -18, razon = 4, ataque = 0.03, soltar = 0.1, sube = 3 },
    { nombre = "Pegamento de mezcla", umbral = -14, razon = 2, ataque = 0.03, soltar = 0.3, sube = 2, oye = 120 },
    { nombre = "Aplastar", umbral = -35, razon = 12, ataque = 0.002, soltar = 0.08, sube = 12, mezcla = 0.5 },
  },
  sonido = function(e, c)
    local nivel = seguidor(maximo(abs(pasaaltos(e.izq, c.oye, 0)), abs(pasaaltos(e.der, c.oye, 0))), c.ataque, c.soltar)
    local g = mide("reduccion", maximo(nivel / db(c.umbral), 1) ^ (1 / c.razon - 1)) * db(c.sube)
    return mezclar(e.izq, e.izq * g, c.mezcla), mezclar(e.der, e.der * g, c.mezcla)
  end,
  interfaz = equipo.rack{ tinte = "#3a4a66", color = "#ffd95e", metal = "#b4b7bd", aguja = function(c, m) return equipo.baja(m), m.idioma == "en" and "REDUCTION" or "REDUCCIÓN" end, claves = { "umbral", "razon", "ataque", "soltar", "oye", "sube", "mezcla" } },
}
