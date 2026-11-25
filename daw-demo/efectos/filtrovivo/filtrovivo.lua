-- Filtro vivo: se toca con el dedo en el cuadro (frecuencia y filo) y se mueve solo al ritmo de la canción.
return {
  nombre = { es = "Filtro XY", en = "XY filter" }, tipo = "efecto", grupo = { es = "Tono", en = "Tone" },
  controles = {
    { "x", { es = "Frecuencia", en = "Frequency" }, 0, 1, 0.8 },
    { "y", { es = "Filo", en = "Resonance" }, 0, 0.95, 0.2 },
    { "tipo", { es = "Tipo", en = "Type" }, 0, 2, 0 },
    { "mueve", { es = "Se mueve", en = "Movement" }, 0, 0.5, 0 },
    { "cada", { es = "Negras por vuelta", en = "Beats per cycle" }, 0.25, 8, 1 },
  },
  presets = {
    { nombre = { es = "Cerrado", en = "Closed" }, x = 0.35, y = 0.3 },
    { nombre = { es = "Vaivén", en = "Sweep" }, x = 0.55, y = 0.6, mueve = 0.3, cada = 4 },
    { nombre = { es = "Guau a tempo", en = "Wah in time" }, x = 0.55, y = 0.8, tipo = 2, mueve = 0.2, cada = 1 },
  },
  sonido = function(e, c)
    local lfo = sen(posicion() / c.cada)
    local donde = minimo(maximo(c.x + lfo * c.mueve, 0), 1)
    local hz = exp(donde * math.log(450)) * 40
    local function es(k) return mayor(c.tipo, k - 0.5) * mayor(k + 0.5, c.tipo) end
    local function f(x)
      return pasabajos(x, hz, c.y) * es(0) + pasaaltos(x, hz, c.y) * es(1) + pasabanda(x, hz, c.y) * es(2)
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = {
    ancho = 380, alto = 260,
    dibujar = function(d, c, m)
      d.acero(0, 0, 380, 260, "#405030", 10)
      d.grabado(30, 20, m.titulo, 20, "#b6f25e")
      d.hueco(14, 38, 260, 208, 6)
      for i = 1, #m.espectro do
        local h = m.espectro[i] * 200
        d.caja(16 + (i - 1) * 4, 244 - h, 3, h, "#b6f25e38")
      end
      local px, py = 14 + c.x * 260, 246 - c.y / 0.95 * 208
      d.linea({ px, 38, px, 246 }, "#b6f25e66", 1)
      d.linea({ 14, py, 274, py }, "#b6f25e66", 1)
      d.circulo(px, py, 9, "#b6f25e")
      d.pad(14, 38, 260, 208, "x", "y")
      local nombres = m.idioma == "en" and { "Low", "High", "Band" } or { "Graves", "Agudos", "Banda" }
      d.lista(288, 38, 78, 88, "tipo", nombres, "#b6f25e")
      d.perilla(327, 160, 20, "mueve", "#b6f25e")
      d.grabado(327, 188, m.idioma == "en" and "Movement" or "Se mueve", 12, "#d6e6c8", "centro")
      d.perilla(327, 216, 20, "cada", "#b6f25e")
      d.grabado(327, 244, m.idioma == "en" and "Beats" or "Negras", 12, "#d6e6c8", "centro")
    end,
  },
}
