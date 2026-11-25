local equipo = incluir("equipo")

return {
  nombre = { es = "Moldeador de transitorios", en = "Transient shaper" }, tipo = "efecto", grupo = "Dinámica",
  controles = { { "golpe", "Golpe", -1, 1, 0.4 }, { "cola", "Cola", -1, 1, 0 },
    { "corte", "Desde Hz", 20, 8000, 20 }, { "empuje", "Saturación", 0, 1, 0 }, { "sale", "Salida dB", -18, 18, 0 } },
  presets = {
    { nombre = "Más golpe", golpe = 0.7, cola = 0 },
    { nombre = "Más seco", golpe = 0.2, cola = -0.7 },
    { nombre = "Más cola", golpe = -0.3, cola = 0.7 },
  },
  sonido = function(e, c)
    local x = maximo(abs(e.izq), abs(e.der))
    local rapido, lento = seguidor(x, 0.001, 0.05), seguidor(x, 0.03, 0.05)
    local d = maximo(rapido - lento, 0) / (rapido + 0.0001)
    local largo = seguidor(x, 0.001, 0.4)
    local s = maximo(largo - rapido, 0) / (largo + 0.0001)
    local g = maximo(1 + d * c.golpe * 2 + s * c.cola * 1.5, 0)
    local function f(x)
      -- Solo moldea lo que está arriba del corte.
      local alto = si(mayor(c.corte, 21), pasaaltos(x, c.corte, 0), x)
      local y = (x - alto) + alto * g
      return mezclar(y, curva(y * 2, function(v) return math.tanh(v) end, "suave") * 0.7, c.empuje) * db(c.sale)
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = equipo.rack{ tinte = "#74552a", color = "#ffd98a", metal = "#c9b58a", claves = { "golpe", "cola", "corte", "empuje", "sale" } },
}
