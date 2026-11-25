-- Cambia tono: sube o baja lo que entra, en vivo, sin cambiar la velocidad.
return {
  nombre = { es = "Cambiador de tono", en = "Pitch shifter" }, tipo = "efecto", grupo = { es = "Tono", en = "Tone" },
  controles = {
    { "semi", { es = "Semitonos", en = "Semitones" }, -12, 12, 7 },
    { "fino", { es = "Fino", en = "Fine" }, -1, 1, 0 },
    { "mezcla", { es = "Mezcla", en = "Mix" }, 0, 1, 0.5 },
  },
  presets = {
    { nombre = { es = "Octava abajo", en = "Octave down" }, semi = -12, mezcla = 0.5 },
    { nombre = { es = "Quinta arriba", en = "Fifth up" }, semi = 7, mezcla = 0.45 },
    { nombre = { es = "Doblar la voz", en = "Voice doubler" }, semi = 0, fino = 0.12, mezcla = 0.4 },
    { nombre = { es = "Ardilla", en = "Chipmunk" }, semi = 8, mezcla = 1 },
  },
  sonido = function(e, c)
    local ventana = 0.05
    local razon = 2 ^ ((piso(c.semi + 0.5) + c.fino) / 12)
    local hz = abs(1 - razon) / ventana
    local sube = mayor(razon, 1)
    -- Dos lectores que se turnan: mientras uno salta, el otro se oye.
    local function lector(x, fase)
      local s = (sierra(hz, fase) + 1) * 0.5
      local atras = si(sube, 1 - s, s) * ventana + 0.002
      return retardo(x, atras, 0, 0.06) * sen(s * 0.5)
    end
    local function f(x)
      local a, b = lector(x, 0), lector(x, 0.5)
      return mezclar(x, a + b, c.mezcla)
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = {
    ancho = 360, alto = 190,
    dibujar = function(d, c, m)
      d.acero(0, 0, 360, 190, "#2e4474", 10)
      d.grabado(30, 22, m.titulo, 20, "#7fb8ff")
      local s = math.floor(c.semi + 0.5)
      d.hueco(16, 40, 120, 66, 6)
      d.texto(76, 73, (s > 0 and "+" or "") .. s, 44, "#bfe0ff", "centro")
      -- Un teclado chico: dónde cae la nota que sale si entra un do.
      d.bandeja(11, 113, 338, 44, "#0c1220", 6)
      for i = -12, 12 do
        local x = 16 + (i + 12) * 13.2
        local negra = ({ [1] = true, [3] = true, [6] = true, [8] = true, [10] = true })[i % 12]
        d.caja(x, 118, 11, negra and 24 or 34, i == s and "#7fb8ff" or (negra and "#4a5468" or "#dfe4ec"), 2)
        if m.clic and m.x and m.x >= x and m.x < x + 13.2 and m.y >= 118 and m.y < 152 then d.poner("semi", i) end
      end
      local en = m.idioma == "en"
      d.perilla(200, 70, 26, "fino", "#7fb8ff")
      d.grabado(200, 106, en and "Fine" or "Fino", 13, "#cfe0f5", "centro")
      d.perilla(290, 70, 26, "mezcla", "#7fb8ff")
      d.grabado(290, 106, en and "Mix" or "Mezcla", 13, "#cfe0f5", "centro")
      d.grabado(180, 172, en and "Click a key to choose the interval" or "Toca una tecla para elegir el intervalo", 12, "#cfe0f5", "centro")
    end,
  },
}
