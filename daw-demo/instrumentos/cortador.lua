-- Cortador: parte un ritmo en pedazos, uno por tecla desde el do de la tercera octava (nota 48).
local equipo = incluir("equipo")

return {
  muestras = "cortes",
  nombre = { es = "Cortador de ritmos", en = "Loop slicer" }, grupo = { es = "Muestras", en = "Samples" }, voces = 8,
  controles = {
    { "donde", { es = "Dónde cortar", en = "Where to cut" }, 0, 1, 1 }, { "partes", { es = "Partes", en = "Parts" }, 2, 32, 16 }, { "fino", { es = "Sensibilidad", en = "Sensitivity" }, 0, 1, 0.5 },
    { "tono", "Tono", -24, 24, 0 }, { "a", { es = "Ataque", en = "Attack" }, 0.0005, 0.2, 0.0005 }, { "cae", { es = "Caída", en = "Decay" }, 0.02, 4, 4 }, { "baja", "Soltar", 0.005, 1, 0.02 },
    { "corte", "Corte", 100, 18000, 18000 },
  },
  sonido = function(n, c)
    local v = n.hz / 261.6256 * 2 ^ (piso(c.tono + 0.5) / 12)
    local e = adsr(c.a, 0.1, 1, c.baja) * mezclar(golpe(c.cae), 1, mayor(c.cae, 3.95)) * n.fuerza
    return pasabajos(muestra(v, "izq"), c.corte, 0) * e, pasabajos(muestra(v, "der"), c.corte, 0) * e
  end,
  interfaz = {
    ancho = 660, alto = 380,
    dibujar = function(d, c, m)
      local en = m.idioma == "en"
      d.acero(0, 0, 660, 380, "#4a5c48", 10)
      d.grabado(30, 24, m.titulo, 21, "#b8f08a")
      d.archivo(250, 12, 180, 26, en and "Load loop…" or "Cargar ritmo…")
      d.escribir(440, 12, 200, 26, en and "Write to pattern" or "Escribir en el patrón")
      d.hueco(22, 52, 616, 130, 8)
      for i = 1, #m.huella do
        local x, h = 24 + (i - 1) * 2.4, m.huella[i] * 60
        d.linea({ x, 117 - h, x, 117 + h }, "#b8f08a", 1.6)
      end
      for i, p in ipairs(m.cortes) do
        local x = 24 + p * 612
        d.linea({ x, 54, x, 180 }, "#ffd95e", 1.2)
        d.texto(x + 3, 62, tostring(i), 9, "#ffd95e")
      end
      d.lista(30, 198, 220, 28, "donde", en and { "Equal parts", "At the hits" } or { "Partes iguales", "En los golpes" }, "#b8f08a")
      d.bandeja(22, 240, 616, 64, nil, 8)
      equipo.perillas(d, c, m, { x = 26, y = 238, w = 608, claves = { "partes", "fino", "tono", "a", "cae", "baja", "corte" }, color = "#b8f08a", metal = "#8d8f96", alto = 64, r = 14 })
      d.teclado(26, 316, 608, 50, 48, 25)
    end,
  },
}
