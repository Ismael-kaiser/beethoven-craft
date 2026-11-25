local equipo = incluir("equipo")

-- Recortador suave: redondea lo que pasa del umbral en vez de cortarlo.
return {
  nombre = { es = "Recortador suave", en = "Soft clipper" }, tipo = "efecto", grupo = "Dinámica",
  controles = { { "umbral", "Umbral dB", -24, 0, -6 }, { "entra", "Entrada dB", -12, 24, 0 }, { "sale", "Salida dB", -24, 12, 0 } },
  presets = { { nombre = { es = "Proteger la mezcla", en = "Protect the mix" }, umbral = -3 }, { nombre = { es = "Batería apretada", en = "Tight drums" }, umbral = -10, entra = 6 }, { nombre = { es = "Aplastado", en = "Crushed" }, umbral = -18, entra = 12, sale = 6 } },
  sonido = function(e, c)
    local u = db(c.umbral)
    local function f(x)
      x = x * db(c.entra)
      local a = abs(x)
      -- Debajo del umbral pasa igual; arriba se acerca a 1 sin llegar.
      local k = (a - u) / (1.0001 - u)
      local y = u + (1.0001 - u) * (k / (1 + maximo(k, 0)))
      return x * mide("reduccion", si(mayor(a, u), y / (a + 0.00001), 1)) * db(c.sale)
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = equipo.rack{ tinte = "#7a4a30", color = "#ffc890", metal = "#8d8f96", aguja = function(c, m) return equipo.baja(m), m.idioma == "en" and "REDUCTION" or "REDUCCIÓN" end, claves = { "entra", "umbral", "sale" } },
}
