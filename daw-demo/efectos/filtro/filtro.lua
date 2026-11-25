local equipo = incluir("equipo")

return {
  nombre = { es = "Filtro de paso alto y bajo", en = "High and low pass filter" }, tipo = "efecto", grupo = "Tono",
  controles = { { "alto", "Quita graves", 15, 2000, 15 }, { "bajo", "Quita agudos", 200, 18000, 18000 }, { "filo", "Resonancia", 0, 0.9, 0 }, { "doble", "Más inclinado", 0, 1, 0 } },
  presets = {
    { nombre = "Quitar retumbe", alto = 90 },
    { nombre = "Detrás de la pared", bajo = 700 },
    { nombre = "Radio", alto = 400, bajo = 3500, filo = 0.4 },
  },
  sonido = function(e, c)
    local function f(x)
      local y = pasabajos(pasaaltos(x, c.alto, c.filo), c.bajo, c.filo)
      return si(c.doble, pasabajos(pasaaltos(y, c.alto, 0.3), c.bajo, 0.3), y)
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = equipo.rack{ tinte = "#3f5238", color = "#c8f08a", metal = "#8d8f96", claves = { "alto", "bajo", "filo", "doble" } },
}
