local equipo = incluir("equipo")

-- Reverb de convolución: pone el sonido dentro de un lugar, o de cualquier audio que le cargues.
return {
  nombre = { es = "Reverb de convolución", en = "Convolution reverb" }, tipo = "efecto", grupo = "Espacio",
  controles = {
    { "espacio", { es = "Lugar", en = "Space" }, 0, 7, 2 }, { "largo", { es = "Largo", en = "Length" }, 0.02, 1, 1 }, { "reves", { es = "Al revés", en = "Reverse" }, 0, 1, 0 },
    { "antes", "Antes", 0, 0.25, 0 }, { "corte", "Quita graves", 20, 1000, 60 }, { "tope", "Quita agudos", 1000, 20000, 16000 },
    { "ancho", "Ancho", 0, 1.5, 1 }, { "mezcla", "Mezcla", 0, 1, 0.3 }, { "seco", "Seco", 0, 1, 1 },
  },
  presets = {
    { nombre = { es = "Voz en cabina", en = "Vocal booth" }, espacio = 0, mezcla = 0.3 }, { nombre = { es = "Batería en estudio", en = "Studio drums" }, espacio = 1, mezcla = 0.35 },
    { nombre = { es = "Coro de catedral", en = "Cathedral choir" }, espacio = 3, mezcla = 0.5, antes = 0.04 }, { nombre = { es = "Placa de voz", en = "Vocal plate" }, espacio = 4, mezcla = 0.25, corte = 250 },
    { nombre = { es = "Guitarra con resorte", en = "Spring guitar" }, espacio = 5, mezcla = 0.35 }, { nombre = { es = "Entrada al revés", en = "Reverse swell" }, espacio = 2, reves = 1, largo = 0.4, mezcla = 0.6 },
    { nombre = { es = "Radio vieja", en = "Old radio" }, espacio = 7, mezcla = 1, seco = 0 },
  },
  sonido = function(e, c)
    local function f(x, lado) return pasaaltos(pasabajos(convolucion(retardo(x, c.antes, 0, 0.26), lado), c.tope, 0), c.corte, 0) end
    local a, b = f(e.izq, "izq"), f(e.der, "der")
    local m, l = (a + b) * 0.5, (a - b) * 0.5 * c.ancho
    return e.izq * c.seco + (m + l) * c.mezcla, e.der * c.seco + (m - l) * c.mezcla
  end,
  interfaz = {
    ancho = 680, alto = 362,
    dibujar = function(d, c, m)
      local en = m.idioma == "en"
      d.madera(0, 0, 22, 362) d.madera(658, 0, 22, 362)
      d.acero(20, 0, 640, 362, "#3a4250", 3)
      d.grabado(44, 25, m.titulo, 20, "#8fe0ff")
      d.prender(628, 25, 10, "#8fe0ff", "palanca")
      d.hueco(48, 48, 520, 128, 8)
      -- La huella del lugar, tal como se va a usar.
      local n = #m.huella
      local alto = 0.000001
      for i = 1, n do alto = math.max(alto, m.huella[i]) end
      local v = {}
      for i = 1, n do
        local h = (m.huella[i] / alto) ^ 0.5 * 58
        local x = 52 + (i - 1) * 2
        d.linea({ x, 112 - h, x, 112 + h }, "#8fe0ff", 1.4)
      end
      local lugares = en and { "Booth", "Studio", "Concert hall", "Cathedral", "Plate", "Spring", "Bathroom", "Radio" } or { "Cabina", "Estudio", "Sala", "Catedral", "Placa", "Resorte", "Baño", "Radio" }
      d.texto(558, 62, m.archivo ~= "" and m.archivo or lugares[math.floor(c.espacio + 0.5) + 1], 13, "#e6f8ff", "derecha")
      d.lista(48, 190, 520, 28, "espacio", lugares, "#8fe0ff")
      d.archivo(578, 188, 62, 32, en and "Load…" or "Cargar…")
      d.palanca(612, 138, "reves", "#8fe0ff")
      d.grabado(612, 170, m.nombres.reves, 11, "#f2efe6", "centro")
      d.bandeja(44, 232, 592, 114, nil, 8)
      equipo.perillas(d, c, m, { x = 50, y = 240, w = 580, claves = { "largo", "antes", "corte", "tope", "ancho", "mezcla", "seco" }, color = "#8fe0ff", metal = "#b4b7bd", alto = 100, r = 27 })
    end,
  },
}
