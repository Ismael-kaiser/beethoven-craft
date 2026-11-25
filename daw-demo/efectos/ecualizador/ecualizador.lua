-- Seis bandas, cada una del tipo que quieras, y cortes de graves y agudos. En el Mixer se mueve con la curva.
local equipo = incluir("equipo")

return {
  nombre = { es = "Ecualizador paramétrico", en = "Parametric EQ" }, tipo = "efecto", grupo = "Tono",
  controles = {
    { "h1", "Graves Hz", 20, 500, 80 }, { "g1", "Graves dB", -18, 18, 0 }, { "q1", "Graves ancho", 0.3, 4, 0.7 },
    { "h2", "2 Hz", 40, 2000, 200 }, { "g2", "2 dB", -18, 18, 0 }, { "q2", "2 ancho", 0.3, 12, 1 },
    { "h3", "3 Hz", 100, 5000, 600 }, { "g3", "3 dB", -18, 18, 0 }, { "q3", "3 ancho", 0.3, 12, 1 },
    { "h4", "4 Hz", 200, 10000, 1800 }, { "g4", "4 dB", -18, 18, 0 }, { "q4", "4 ancho", 0.3, 12, 1 },
    { "h5", "5 Hz", 500, 16000, 4500 }, { "g5", "5 dB", -18, 18, 0 }, { "q5", "5 ancho", 0.3, 12, 1 },
    { "h6", "Agudos Hz", 1000, 18000, 9000 }, { "g6", "Agudos dB", -18, 18, 0 }, { "q6", "Agudos ancho", 0.3, 4, 0.7 },
    { "t1", "Tipo 1", 0, 3, 1 }, { "t2", "Tipo 2", 0, 3, 0 }, { "t3", "Tipo 3", 0, 3, 0 }, { "t4", "Tipo 4", 0, 3, 0 }, { "t5", "Tipo 5", 0, 3, 0 }, { "t6", "Tipo 6", 0, 3, 2 },
    { "corta", "Corta graves Hz", 20, 1000, 20 }, { "tope", "Corta agudos Hz", 1000, 20000, 20000 },
    { "sale", "Salida dB", -18, 18, 0 },
  },
  presets = {
    { nombre = "Voz clara", h1 = 90, g1 = -12, g2 = -2, h4 = 3000, g4 = 3, g6 = 2 },
    { nombre = "Más graves", h1 = 90, g1 = 5, g2 = 2 },
    { nombre = "Más brillo", g5 = 2, g6 = 5 },
    { nombre = "Teléfono", h1 = 450, g1 = -18, g4 = 6, h6 = 3500, g6 = -18 },
  },
  sonido = function(e, c)
    local function banda(x, i)
      local t, hz, g, q = c["t" .. i], c["h" .. i], c["g" .. i], c["q" .. i]
      local function es(k) return mayor(t, k - 0.5) * mayor(k + 0.5, t) end
      return campana(x, hz, g, q) * es(0) + graves(x, hz, g, q) * es(1) + agudos(x, hz, g, q) * es(2) + muesca(x, hz, q * 2) * es(3)
    end
    local function f(x)
      for i = 1, 6 do x = banda(x, i) end
      x = si(mayor(c.corta, 20.5), pasaaltos(x, c.corta, 0.3), x)
      x = si(mayor(19900, c.tope), pasabajos(x, c.tope, 0.3), x)
      return x * db(c.sale)
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = {
    ancho = 760, alto = 500,
    dibujar = function(d, c, m)
      local en = m.idioma == "en"
      d.acero(0, 0, 760, 500, "#3e4858", 9)
      d.grabado(34, 24, m.titulo, 20, "#ffd95e")
      d.prender(728, 60, 12, "#ffd95e", "palanca")
      d.ecualizador(22, 46, 678, 270)
      d.bandeja(20, 330, 720, 154, "#2a2c30", 8)
      local tonos = { "#ff8a6a", "#ffc85e", "#a8e070", "#5ed8c8", "#7aaeff", "#c890ff" }
      local tipos = en and { "Bell", "Low shelf", "High shelf", "Notch" } or { "Campana", "Graves", "Agudos", "Muesca" }
      for i = 1, 6 do
        local x = 26 + (i - 1) * 95
        d.rota(x, 337, 89, 25, "t" .. i, tipos, tonos[i])
        for k, q in ipairs({ { "h", 22, 388, 17 }, { "q", 67, 388, 17 }, { "g", 44, 440, 23 } }) do
          d.perilla(x + q[2], q[3], q[4], q[1] .. i, tonos[i], "#8c8e94")
          d.texto(x + q[2], q[3] + q[4] + 7, equipo.numero(c[q[1] .. i]), 10, tonos[i], "centro")
        end
      end
      equipo.perillas(d, c, m, { x = 600, y = 336, w = 136, claves = { "corta", "tope", "sale" }, por = 2, color = "#ffd95e", metal = "#8c8e94", alto = 72, r = 17 })
    end,
  },
}
