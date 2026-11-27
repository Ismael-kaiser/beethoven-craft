local equipo = incluir("equipo")

-- Afinador de voz: oye qué nota se canta y la lleva a la más cercana de la escala, en vivo.
return {
  nombre = { es = "Afinador de voz", en = "Vocal tuner" }, tipo = "efecto", grupo = "Voz",
  controles = {
    { "tono", { es = "Tonalidad", en = "Key" }, 0, 11, 0 }, { "escala", { es = "Escala", en = "Scale" }, 0, 2, 0 },
    { "rapido", { es = "Rapidez", en = "Speed" }, 0, 1, 0.7 }, { "sube", { es = "Semitonos", en = "Semitones" }, -12, 12, 0 }, { "mezcla", "Mezcla", 0, 1, 1 },
  },
  presets = { { nombre = { es = "Natural", en = "Natural" }, rapido = 0.35 }, { nombre = { es = "Robot", en = "Robot" }, rapido = 1 }, { nombre = { es = "Octava abajo", en = "Octave down" }, sube = -12 } },
  medidas = { "nota", "meta", "voz" },
  vivo = function(v)
    local escalas = { { 0, 2, 4, 5, 7, 9, 11 }, { 0, 2, 3, 5, 7, 8, 10 }, { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 } }
    -- Para oír la nota: a un cuarto de la frecuencia, con la autocorrelación.
    local n, salto = 1024, 4
    local oye, re, im, k, junta, cuenta = {}, {}, {}, 0, 0.0, 0
    for i = 1, n do oye[i] = 0.0 end
    for i = 1, n * 2 do re[i], im[i] = 0.0, 0.0 end
    local sr = v.sr / salto
    local largo = math.floor(v.sr * 0.1)
    local linea = { retraso(largo), retraso(largo) }
    local razon, meta, fase, periodo = 1.0, 1.0, 0.0, v.sr / 200
    local espera = 0
    local function detecta(c)
      local e = 0.0
      for i = 1, n do
        local x = oye[(k + i - 1) % n + 1]
        re[i], im[i] = x, 0.0
        re[n + i], im[n + i] = 0.0, 0.0
        e = e + x * x
      end
      if e < 0.02 then mide("voz", 0) meta = 1.0 return end
      fft(re, im)
      for i = 1, n * 2 do
        re[i] = re[i] * re[i] + im[i] * im[i]
        im[i] = 0.0
      end
      fft(re, im, true)
      local desde, hasta = math.floor(sr / 900), math.min(math.floor(sr / 65), n - 2)
      local mejor, donde = 0.0, 0
      local e0 = re[1]
      for i = desde, hasta do
        local x = re[i + 1] / (1 - i / n)
        if x > mejor and re[i + 1] > re[i] and re[i + 1] >= re[i + 2] then
          -- Un poco de ventaja a la primera: así no baja una octava.
          if x > mejor * 1.06 then mejor, donde = x, i end
        end
      end
      if donde == 0 or mejor < e0 * 0.45 then mide("voz", 0) meta = 1.0 return end
      local a, b, d = re[donde] / (1 - (donde - 1) / n), re[donde + 1] / (1 - donde / n), re[donde + 2] / (1 - (donde + 1) / n)
      local den = a - 2 * b + d
      local fino = donde + (math.abs(den) > 1e-9 and 0.5 * (a - d) / den or 0)
      periodo = fino * salto
      local nota = 69 + 12 * math.log(v.sr / periodo / 440, 2)
      local mejor_n, dist = nota, 99
      local e_ = escalas[math.floor(c.escala + 0.5) + 1]
      for o = math.floor(nota / 12) - 1, math.floor(nota / 12) + 1 do
        for i = 1, #e_ do
          local x = o * 12 + e_[i] + math.floor(c.tono + 0.5)
          if math.abs(x - nota) < dist then mejor_n, dist = x, math.abs(x - nota) end
        end
      end
      meta = 2 ^ ((mejor_n - nota) / 12)
      mide("nota", nota) mide("meta", mejor_n) mide("voz", 1)
    end
    return function(l, r, c, m)
      junta = junta + (l + r) * 0.5
      cuenta = cuenta + 1
      if cuenta == salto then
        oye[k % n + 1] = junta / salto
        k = k + 1
        junta, cuenta = 0.0, 0
        if k % 128 == 0 then detecta(c) end
      end
      local q = c.rapido * c.rapido
      razon = razon + (meta - razon) * (q * 0.02 + 0.00008)
      local total = razon * 2 ^ (c.sube / 12)
      -- Dos lectores que se turnan cada dos vueltas de la voz: por eso no se oye el salto.
      -- Siempre un número par de vueltas: si no, al turnarse se pierde la afinación.
      local ventana = math.min(periodo * 2 * math.ceil(v.sr * 0.004 / periodo), largo * 0.9)
      fase = (fase + (1 - total) / ventana) % 1
      linea[1].pon(l) linea[2].pon(r)
      local f2 = (fase + 0.5) % 1
      local a, b = math.sin(math.pi * fase), math.sin(math.pi * f2)
      a, b = a * a, b * b
      local base = 4
      local yl = linea[1].lee(base + fase * ventana) * a + linea[1].lee(base + f2 * ventana) * b
      local yr = linea[2].lee(base + fase * ventana) * a + linea[2].lee(base + f2 * ventana) * b
      return l + (yl - l) * c.mezcla, r + (yr - r) * c.mezcla
    end
  end,
  interfaz = {
    ancho = 560, alto = 300,
    dibujar = function(d, c, m)
      local en = m.idioma == "en"
      d.acero(0, 0, 560, 300, "#6a3f5a", 10)
      d.grabado(34, 25, m.titulo, 20, "#ffa8d8")
      d.prender(530, 25, 10, "#ffa8d8", "palanca")
      d.hueco(24, 50, 460, 76, 8)
      local nombres = en and { "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B" } or { "Do", "Do#", "Re", "Re#", "Mi", "Fa", "Fa#", "Sol", "Sol#", "La", "La#", "Si" }
      local nota, meta, voz = m.medidas.nota or 0, m.medidas.meta or 0, (m.medidas.voz or 0) > 0.5 and m.tocando
      for i = -10, 10 do d.linea({ 254 + i * 20, 104, 254 + i * 20, i % 5 == 0 and 92 or 98 }, "#ffffff40", 1) end
      if voz then
        local x = 254 + math.max(-1, math.min(1, nota - meta)) * 200
        d.linea({ x, 62, x, 112 }, math.abs(nota - meta) < 0.15 and "#8ae08f" or "#ffd95e", 3)
        d.texto(46, 88, nombres[math.floor(meta + 0.5) % 12 + 1], 30, "#ffa8d8")
      else
        d.texto(254, 76, en and "Sing something" or "Canta algo", 14, "#8a7684", "centro")
      end
      d.lista(24, 140, 510, 26, "tono", nombres, "#ffa8d8")
      d.lista(24, 176, 300, 26, "escala", en and { "Major", "Minor", "All notes" } or { "Mayor", "Menor", "Todas" }, "#ffa8d8")
      d.bandeja(20, 212, 520, 76, nil, 8)
      equipo.perillas(d, c, m, { x = 24, y = 213, w = 512, claves = { "rapido", "sube", "mezcla" }, color = "#ffa8d8", metal = "#b4b7bd", alto = 72, r = 18 })
    end,
  },
}
