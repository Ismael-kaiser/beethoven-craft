local equipo = incluir("equipo")

return {
  nombre = { es = "Reverb de sala", en = "Room reverb" }, tipo = "efecto", grupo = "Espacio",
  controles = { { "tam", "Tamaño", 0, 1, 0.6 }, { "brillo", "Brillo", 0, 1, 0.5 }, { "mezcla", "Mezcla", 0, 1, 0.3 }, { "seco", "Seco", 0, 1, 1 },
    { "antes", "Antes", 0, 0.2, 0 }, { "corte", "Quita graves", 20, 1000, 20 }, { "tope", "Quita agudos", 1000, 18000, 18000 },
    { "ancho", "Ancho", 0, 1.5, 1 },
    { "primeras", "Primeros rebotes", 0, 1, 0 }, { "cuerpo", "Graves dB", -12, 12, 0 }, { "mueve", "Movimiento", 0, 1, 0 } },
  presets = {
    { nombre = "Cuarto chico", tam = 0.25, brillo = 0.45, mezcla = 0.2, ancho = 0.8 },
    { nombre = "Voz en estudio", tam = 0.5, brillo = 0.6, mezcla = 0.22, antes = 0.035, corte = 250, tope = 9000 },
    { nombre = "Salón", tam = 0.75, brillo = 0.5, mezcla = 0.35, antes = 0.02 },
    { nombre = "Catedral", tam = 0.98, brillo = 0.35, mezcla = 0.5, antes = 0.06, ancho = 1.3 },
  },
  sonido = function(e, c)
    local izq, der = retardo(e.izq, c.antes, 0, 0.21), retardo(e.der, c.antes, 0, 0.21)
    local function limpia(x) return graves(pasaaltos(pasabajos(x, c.tope, 0), c.corte, 0), 220, c.cuerpo, 0.7) end
    -- Los primeros rebotes: las paredes, antes de la cola.
    local function paredes(x, k)
      local y = retardo(x, 0.011 * k, 0, 0.1)
      for i, t in ipairs({ 0.017, 0.023, 0.031, 0.043, 0.059 }) do y = y + retardo(x, t * k, 0, 0.1) * (0.8 - i * 0.11) end
      return y * 0.5 * c.primeras
    end
    local vaiven = 0.004 + seno(0.6) * c.mueve * 0.002
    local a = limpia(sala(si(mayor(c.mueve, 0.001), retardo(izq, vaiven, 0, 0.01), izq) + der * 0.3, c.tam, c.brillo)) + paredes(izq, 1)
    local b = limpia(sala(retardo(der, 0.007) + izq * 0.3, c.tam, c.brillo)) + paredes(der, 1.13)
    local m, l = (a + b) * 0.5, (a - b) * 0.5 * c.ancho
    return e.izq * c.seco + (m + l) * c.mezcla, e.der * c.seco + (m - l) * c.mezcla
  end,
  interfaz = equipo.rack{ tinte = "#3c4658", color = "#9fd4ff", metal = "#b4b7bd", por = 6, claves = { "tam", "brillo", "antes", "primeras", "mueve", "ancho", "corte", "tope", "cuerpo", "mezcla", "seco" } },
}
