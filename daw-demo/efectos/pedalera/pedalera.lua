-- Pedalera: compuerta, saturación, tono y bocina, como tres pedales en fila.
return {
  nombre = { es = "Pedales de guitarra", en = "Guitar pedals" }, tipo = "efecto", grupo = { es = "Carácter", en = "Character" },
  controles = {
    { "callar", { es = "Callar ruido", en = "Gate" }, 0, 1, 0.1 },
    { "fuego", { es = "Saturación", en = "Drive" }, 0, 1, 0.5 },
    { "tono", { es = "Tono", en = "Tone" }, 0, 1, 0.5 },
    { "bocina", { es = "Bocina", en = "Cabinet" }, 0, 1, 1 },
    { "nivel", { es = "Nivel", en = "Level" }, 0, 1, 0.6 },
    { "p1", { es = "Pedal 1", en = "Pedal 1" }, 0, 1, 1 }, { "p2", { es = "Pedal 2", en = "Pedal 2" }, 0, 1, 1 }, { "p3", { es = "Pedal 3", en = "Pedal 3" }, 0, 1, 1 },
  },
  presets = {
    { nombre = { es = "Apenas sucio", en = "Edge of breakup" }, fuego = 0.25, tono = 0.55, nivel = 0.7 },
    { nombre = { es = "Rock", en = "Rock" }, fuego = 0.6, tono = 0.55, callar = 0.2 },
    { nombre = { es = "Muro", en = "Wall" }, fuego = 1, tono = 0.4, callar = 0.35, nivel = 0.45 },
  },
  sonido = function(e, c)
    local function f(x)
      local nivel = seguidor(abs(x), 0.001, 0.08)
      x = mezclar(x, x * minimo(1, (nivel / (c.callar * c.callar * 0.08 + 0.00001)) ^ 4), c.p1)
      local sucio = pasaaltos(x, 90, 0) * (1 + c.fuego * c.fuego * 60)
      -- Disparejo a propósito, como un bulbo.
      sucio = curva(sucio, function(v) return math.tanh(v + 0.25) - math.tanh(0.25) end, "suave")
      sucio = agudos(graves(sucio, 200, 4 - c.tono * 8, 0.7), 1800, c.tono * 16 - 8, 0.7)
      x = mezclar(x, sucio, c.p2)
      local caja = pasabajos(pasabajos(campana(x, 110, 4, 1.2), 5200, 0.2), 6500, 0)
      return mezclar(x, mezclar(x, caja, c.bocina) * c.nivel, c.p3)
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = {
    ancho = 420, alto = 230,
    dibujar = function(d, c, m)
      d.acero(0, 0, 420, 230, "#4a4a52", 10)
      local en = m.idioma == "en"
      local function pedal(x, w, color, titulo, cual)
        d.acero(x, 12, w, 206, color, 10)
        d.grabado(x + w / 2, 30, titulo, 15, "#fff4e0", "centro")
        d.pisoton(x + w / 2, 188, 15, cual)
      end
      local function perilla(x, y, clave, nombre)
        d.perilla(x, y, 24, clave, "#ffe08a", "#c9b58a")
        d.grabado(x, y + 36, nombre, 12, "#f1e9da", "centro")
      end
      pedal(12, 112, "#3d6b57", en and "GATE" or "SILENCIO", "p1")
      perilla(68, 84, "callar", en and "Threshold" or "Umbral")
      pedal(132, 156, "#a1432f", en and "DRIVE" or "FUEGO", "p2")
      perilla(172, 84, "fuego", en and "Drive" or "Saturar")
      perilla(248, 84, "tono", en and "Tone" or "Tono")
      pedal(296, 112, "#3a4a7a", en and "CABINET" or "BOCINA", "p3")
      perilla(326, 84, "bocina", en and "Speaker" or "Caja")
      perilla(380, 84, "nivel", en and "Level" or "Nivel")
    end,
  },
}
