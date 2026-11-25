local equipo = incluir("equipo")

return {
  nombre = { es = "Reductor de bits", en = "Bitcrusher" }, tipo = "efecto", grupo = "Carácter",
  controles = { { "bits", "Bits", 1, 16, 8 }, { "hz", "Muestreo Hz", 500, 44100, 11025 }, { "mezcla", "Mezcla", 0, 1, 1 } },
  presets = {
    { nombre = "Consola vieja", bits = 8, hz = 11025 },
    { nombre = "Teléfono roto", bits = 5, hz = 6000 },
    { nombre = "Ruina", bits = 3, hz = 2500, mezcla = 0.85 },
  },
  sonido = function(e, c)
    local function f(x) return mezclar(x, escalon(retener(x, c.hz), 2 ^ (c.bits - 1)), c.mezcla) end
    return f(e.izq), f(e.der)
  end,
  interfaz = equipo.rack{ tinte = "#2e3230", color = "#7dff9a", metal = "#3c3e42", claves = { "bits", "hz", "mezcla" } },
}
