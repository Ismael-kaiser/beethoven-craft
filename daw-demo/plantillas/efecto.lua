-- Un efecto completo para empezar. Guárdalo, póntelo en una pista y muévelo: los cambios se oyen al guardar.
-- La guía está en daw-demo/PLUGINS.md.
local equipo = incluir("equipo")

return {
  nombre = { es = "Mi efecto", en = "My effect" },
  tipo = "efecto",
  grupo = { es = "Míos", en = "Mine" },
  autor = "", version = "1.0",
  descripcion = { es = "Calienta y redondea el sonido", en = "Warms and rounds the sound" },
  controles = {
    -- clave, nombre, mínimo, máximo, valor
    { "calor", { es = "Calor", en = "Warmth" }, 1, 10, 3 },
    { "tono", { es = "Tono Hz", en = "Tone Hz" }, 300, 18000, 12000 },
    { "cuanto", { es = "Mezcla", en = "Mix" }, 0, 1, 0.5 },
  },
  presets = {
    { nombre = { es = "Suave", en = "Soft" }, calor = 1.5, cuanto = 0.4 },
    { nombre = { es = "Fuerte", en = "Hard" }, calor = 8, tono = 6000, cuanto = 0.9 },
  },
  -- e.izq y e.der es lo que entra; se regresa lo que sale. Corre una vez para armar el sonido, no en cada muestra.
  sonido = function(e, c)
    local function f(x)
      local y = curva(x * c.calor, function(v) return math.tanh(v) end, "suave")
      return mezclar(x, pasabajos(y, c.tono, 0), c.cuanto)
    end
    return f(e.izq), f(e.der)
  end,
  -- Una ventana de aparato en un renglón: también hay equipo.pedal, equipo.cinta y equipo.bulbos.
  interfaz = equipo.rack{ tinte = "#3c5560", color = "#ffd95e", metal = "#8d8f96", que = "onda", claves = { "calor", "tono", "cuanto" } },
}
