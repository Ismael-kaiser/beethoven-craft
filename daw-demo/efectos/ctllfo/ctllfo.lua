local equipo = incluir("equipo")

-- Una onda al ritmo de la canción que mueve cualquier perilla.
-- Vincúlalo: clic derecho en cualquier perilla de un plugin y «Moverlo con…».
return {
  nombre = { es = "Controlador LFO", en = "LFO controller" }, tipo = "efecto", grupo = { es = "Controladores", en = "Controllers" },
  controles = { { "negras", { es = "Negras por vuelta", en = "Beats per cycle" }, 0.25, 32, 4 }, { "forma", { es = "Forma", en = "Shape" }, 0, 3, 0 }, { "fase", { es = "Fase", en = "Phase" }, 0, 1, 0 }, { "min", { es = "Desde", en = "From" }, 0, 1, 0 }, { "max", { es = "Hasta", en = "To" }, 0, 1, 1 } },
  presets = { { nombre = { es = "Lento", en = "Slow" }, negras = 16 }, { nombre = { es = "Bombeo", en = "Pump" }, negras = 1, forma = 2 } },
  sonido = function(e, c)
    local t = resto(posicion() / c.negras + c.fase, 1)
    local function es(k) return mayor(c.forma, k - 0.5) * mayor(k + 0.5, c.forma) end
    local o = (sen(t) * 0.5 + 0.5) * es(0) + (1 - abs(t * 2 - 1)) * es(1) + t * es(2) + mayor(0.5, t) * es(3)
    local x = c.min + (c.max - c.min) * o
    local v = mide("control", minimo(maximo(x, 0), 1))
    -- El sonido pasa igual; el valor se queda en la cuenta sin cambiar nada.
    return e.izq + v * 0, e.der
  end,
  interfaz = {
    ancho = 520, alto = 250,
    dibujar = function(d, c, m)
      d.acero(0, 0, 520, 250, "#4a4a58", 9)
      d.grabado(34, 25, m.titulo, 20, "#b8ffb0")
      d.prender(490, 25, 10, "#b8ffb0", "palanca")
      local v = m.medidas.control or 0
      d.aguja(34, 50, 150, 92, v, string.format("%.0f %%", v * 100))
      for i = 0, 9 do d.led(210 + i * 24, 70, 6, "#b8ffb0", v * 10 > i and 1 or 0) end
      d.lista(206, 118, 240, 26, "forma", m.idioma == "en" and { "Sine", "Triangle", "Saw", "Square" } or { "Seno", "Triángulo", "Sierra", "Cuadrada" }, "#b8ffb0")
      d.grabado(206, 104, m.idioma == "en" and "Right-click any knob: «Move it with…»" or "Clic derecho en una perilla: «Moverlo con…»", 12, "#f2efe6")
      d.bandeja(24, 156, 472, 82, nil, 8)
      equipo.perillas(d, c, m, { x = 28, y = 160, w = 464, claves = { "negras", "fase", "min", "max" }, color = "#b8ffb0", metal = "#8d8f96", alto = 78, r = 18 })
    end,
  },
}
