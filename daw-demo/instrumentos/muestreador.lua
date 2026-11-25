-- Muestreador: toca cualquier audio como instrumento, con su envolvente, su filtro y su vibrato.
local equipo = incluir("equipo")

return {
  nombre = { es = "Muestreador", en = "Sampler" }, grupo = { es = "Muestras", en = "Samples" }, voces = 12,
  controles = {
    { "tono", "Tono", -24, 24, 0 }, { "fino", { es = "Fino", en = "Fine" }, -1, 1, 0 }, { "inicio", { es = "Empieza en", en = "Start" }, 0, 0.95, 0 },
    { "a", { es = "Ataque", en = "Attack" }, 0.001, 3, 0.001 }, { "c", { es = "Caída", en = "Decay" }, 0.01, 5, 0.3 }, { "s", { es = "Sostén", en = "Sustain" }, 0, 1, 1 }, { "baja", "Soltar", 0.005, 3, 0.05 },
    { "corte", "Corte", 100, 18000, 18000 }, { "filo", { es = "Resonancia", en = "Resonance" }, 0, 0.95, 0 }, { "abre", { es = "Abre", en = "Env amount" }, 0, 10000, 0 }, { "cierra", { es = "Cierra", en = "Env decay" }, 0.02, 3, 0.3 },
    { "vib", { es = "Vibrato", en = "Vibrato" }, 0, 1, 0 }, { "vel", { es = "Velocidad", en = "Rate" }, 0.1, 12, 5 }, { "empuje", "Saturación", 0, 1, 0 },
  },
  presets = {
    { nombre = { es = "Golpe corto", en = "Short hit" }, c = 0.12, s = 0 }, { nombre = { es = "Colchón", en = "Pad" }, a = 0.6, baja = 1.5, corte = 5000 },
    { nombre = { es = "Lo-fi", en = "Lo-fi" }, corte = 2600, vib = 0.12, vel = 1.2, empuje = 0.5 },
  },
  sonido = function(n, c)
    local v = n.hz / 261.6256 * 2 ^ ((piso(c.tono + 0.5) + c.fino) / 12) * (1 + seno(c.vel) * c.vib * 0.03)
    local e = adsr(c.a, c.c, c.s, c.baja) * n.fuerza
    local hz = c.corte + golpe(c.cierra) * c.abre
    local function f(lado)
      local x = pasabajos(muestra(v, lado), hz, c.filo)
      return mezclar(x, curva(x * 4, function(u) return math.tanh(u) end, "suave") * 0.5, c.empuje) * e
    end
    return f("izq"), f("der")
  end,
  interfaz = {
    ancho = 660, alto = 430,
    dibujar = function(d, c, m)
      local en = m.idioma == "en"
      d.acero(0, 0, 660, 430, "#4e5560", 10)
      d.grabado(30, 24, m.titulo, 21, "#ffb88a")
      d.texto(430, 24, m.archivo ~= "" and m.archivo or (en and "No audio yet" or "Sin audio todavía"), 13, "#f2efe6", "derecha")
      d.archivo(450, 12, 190, 26, en and "Load audio…" or "Cargar audio…")
      d.hueco(22, 50, 616, 100, 8)
      for i = 1, #m.huella do
        local x, h = 24 + (i - 1) * 2.4, m.huella[i] * 46
        d.linea({ x, 100 - h, x, 100 + h }, (i - 1) / 256 < c.inicio and "#5a4438" or "#ffb88a", 1.6)
      end
      d.linea({ 24 + c.inicio * 612, 52, 24 + c.inicio * 612, 148 }, "#ffd95e", 1.5)
      d.bandeja(22, 164, 616, 82, nil, 8)
      equipo.perillas(d, c, m, { x = 26, y = 167, w = 608, claves = { "tono", "fino", "inicio", "a", "c", "s", "baja" }, color = "#ffb88a", metal = "#8d8f96", alto = 78, r = 18 })
      d.bandeja(22, 256, 616, 82, nil, 8)
      equipo.perillas(d, c, m, { x = 26, y = 259, w = 608, claves = { "corte", "filo", "abre", "cierra", "vib", "vel", "empuje" }, color = "#8fd0ff", metal = "#8d8f96", alto = 78, r = 18 })
      d.teclado(26, 352, 608, 62, 48, 37)
    end,
  },
}
