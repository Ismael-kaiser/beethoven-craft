-- Un instrumento completo para empezar. Guárdalo y tócalo con el teclado: los cambios se oyen al guardar.
-- La guía está en daw-demo/PLUGINS.md.
local equipo = incluir("equipo")

return {
  nombre = { es = "Mi sonido", en = "My sound" },
  grupo = { es = "Míos", en = "Mine" },
  autor = "", version = "1.0",
  voces = 12,
  controles = {
    -- clave, nombre, mínimo, máximo, valor
    { "corte", { es = "Corte", en = "Cutoff" }, 100, 12000, 3000 },
    { "abre", { es = "Desafinado", en = "Detune" }, 0, 1, 0.3 },
    { "a", { es = "Ataque", en = "Attack" }, 0.001, 2, 0.01 },
    { "r", { es = "Soltar", en = "Release" }, 0.01, 4, 0.2 },
  },
  presets = { { nombre = { es = "Colchón", en = "Pad" }, a = 0.8, r = 1.5, corte = 1800 } },
  -- n.hz es la nota, n.fuerza qué tan fuerte se tocó y n.azar un número distinto en cada nota.
  sonido = function(n, c)
    local x = sierra(n.hz) + sierra(n.hz * (1 + c.abre * 0.01))
    return pasabajos(x, c.corte, 0.2) * adsr(c.a, 0.3, 0.7, c.r) * n.fuerza * 0.3
  end,
  interfaz = {
    ancho = 520, alto = 260,
    dibujar = function(d, c, m)
      d.acero(0, 0, 520, 260, "#4a5c74", 10)
      d.grabado(30, 25, m.titulo, 21, "#ffd95e")
      d.bandeja(20, 46, 480, 96, nil, 8)
      equipo.perillas(d, c, m, { x = 24, y = 50, w = 472, claves = { "corte", "abre", "a", "r" }, color = "#ffd95e", metal = "#8d8f96", alto = 88, r = 22 })
      d.teclado(24, 158, 472, 86, 48, 25)
    end,
  },
}
