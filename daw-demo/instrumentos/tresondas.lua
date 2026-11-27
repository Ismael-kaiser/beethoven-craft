-- Tres ondas: tres osciladores afinables, un filtro con su envolvente y la envolvente del volumen.

local function fila(d, m, x, y, paso, r, color, lista, metal)
  for i, p in ipairs(lista) do
    local cx = x + (i - 1) * paso
    d.perilla(cx, y, r, p[1], color, metal)
    d.grabado(cx, y + r + 11, m.idioma == "en" and p.en or p.es, 11, "#f4f1ea", "centro")
  end
end
local function marco(d, x, y, w, h, titulo, color)
  d.bandeja(x, y, w, h, nil, 7)
  d.grabado(x + 10, y + 13, titulo, 12, color)
end

return {
  nombre = { es = "Sinte de 3 osciladores", en = "3-oscillator synth" }, grupo = { es = "Sintetizadores propios", en = "Our synths" }, voces = 12,
  controles = {
    { "o1", { es = "Onda 1", en = "Wave 1" }, 0, 3, 1 }, { "o2", { es = "Onda 2", en = "Wave 2" }, 0, 3, 1 }, { "o3", { es = "Onda 3", en = "Wave 3" }, 0, 3, 2 },
    { "s2", { es = "Semitonos 2", en = "Semitones 2" }, -24, 24, 0 }, { "s3", { es = "Semitonos 3", en = "Semitones 3" }, -24, 24, -12 },
    { "fino", { es = "Desafinar", en = "Detune" }, 0, 0.5, 0.08 },
    { "v2", { es = "Nivel 2", en = "Level 2" }, 0, 1, 0.8 }, { "v3", { es = "Nivel 3", en = "Level 3" }, 0, 1, 0.6 },
    { "anillo", { es = "Anillo 1x2", en = "Ring 1x2" }, 0, 1, 0 }, { "ancho", { es = "Estéreo", en = "Stereo" }, 0, 1, 0 }, { "vib", { es = "Vibrato", en = "Vibrato" }, 0, 1, 0 },
    { "corte", { es = "Corte", en = "Cutoff" }, 60, 16000, 2400 }, { "filo", { es = "Filo", en = "Resonance" }, 0, 0.95, 0.2 },
    { "abre", { es = "Abre", en = "Env amount" }, 0, 8000, 2000 }, { "cierra", { es = "Cierra", en = "Env decay" }, 0.02, 3, 0.4 },
    { "a", { es = "Ataque", en = "Attack" }, 0.001, 2, 0.005 }, { "c", { es = "Caída", en = "Decay" }, 0.01, 3, 0.4 },
    { "s", { es = "Sostén", en = "Sustain" }, 0, 1, 0.7 }, { "r", { es = "Soltar", en = "Release" }, 0.005, 4, 0.2 },
  },
  presets = {
    { nombre = { es = "Bajo redondo", en = "Round bass" }, o1 = 1, o2 = 2, o3 = 0, corte = 500, abre = 1500, cierra = 0.2, s3 = -12 },
    { nombre = { es = "Metales gordos", en = "Fat brass" }, o1 = 1, o2 = 1, o3 = 1, fino = 0.16, corte = 1200, abre = 4500, cierra = 0.6, a = 0.06 },
    { nombre = { es = "Colchón lento", en = "Slow pad" }, o1 = 1, o2 = 3, o3 = 0, fino = 0.2, corte = 1800, abre = 800, a = 0.9, r = 1.8, s = 0.9 },
    { nombre = { es = "Cuadrada de juego", en = "Game square" }, o1 = 2, o2 = 2, v2 = 0, v3 = 0, corte = 16000, abre = 0, a = 0.001, s = 0.8, r = 0.03 },
  },
  sonido = function(n, c)
    local function es(v, k) return mayor(v, k - 0.5) * mayor(k + 0.5, v) end
    local function onda_(v, hz)
      return seno(hz) * es(v, 0) + sierra(hz) * es(v, 1) + pulso(hz, 0.5) * es(v, 2) + triangulo(hz) * es(v, 3)
    end
    local function semi(s) return 2 ^ (piso(s + 0.5) / 12) end
    local d = 2 ^ (c.fino / 12)
    local base = n.hz * (1 + seno(5.5) * c.vib * 0.02)
    local uno = onda_(c.o1, base)
    local function lado(k)
      local dos = onda_(c.o2, base * semi(c.s2) * k)
      return uno + dos * c.v2 + onda_(c.o3, base * semi(c.s3) / k) * c.v3 + uno * dos * c.anillo
    end
    local a = lado(d)
    -- El otro lado se desafina al revés: por eso se abre.
    local b = mezclar(a, lado(1 / d), c.ancho)
    local hz = c.corte + golpe(c.cierra) * c.abre
    local e = adsr(c.a, c.c, c.s, c.r) * n.fuerza * 0.16
    return pasabajos(a, hz, c.filo) * e, pasabajos(b, hz, c.filo) * e
  end,
  interfaz = {
    ancho = 520, alto = 330,
    dibujar = function(d, c, m)
      d.acero(0, 0, 520, 330, "#5a6474", 10)
      local en = m.idioma == "en"
      d.grabado(30, 24, m.titulo, 22, "#ffd95e")
      d.onda(300, 10, 196, 28, "#ffd95e")
      local ondas = { "~", "/|", "[]", "/\\" }
      for i = 1, 3 do
        local y = 44 + (i - 1) * 64
        marco(d, 14, y, 330, 58, (en and "OSC " or "OSC ") .. i, "#ffd95e")
        d.lista(24, y + 24, 140, 26, "o" .. i, ondas, "#ffd95e")
        if i > 1 then
          fila(d, m, 214, y + 23, 62, 15, "#ffd95e", { { "s" .. i, es = "Semi", en = "Semi" }, { "v" .. i, es = "Nivel", en = "Level" } })
        else
          fila(d, m, 214, y + 23, 62, 15, "#ffd95e", { { "fino", es = "Desafinar", en = "Detune" }, { "anillo", es = "Anillo", en = "Ring" } })
        end
      end
      marco(d, 354, 44, 152, 186, en and "FILTER" or "FILTRO", "#8fd0ff")
      fila(d, m, 392, 86, 74, 24, "#8fd0ff", { { "corte", es = "Corte", en = "Cutoff" }, { "filo", es = "Filo", en = "Reso" } })
      fila(d, m, 392, 166, 74, 24, "#8fd0ff", { { "abre", es = "Abre", en = "Env" }, { "cierra", es = "Cierra", en = "Decay" } })
      marco(d, 14, 240, 492, 78, en and "VOLUME" or "VOLUMEN", "#8ae08f")
      fila(d, m, 96, 276, 74, 18, "#8ae08f", { { "a", es = "Ataque", en = "Attack" }, { "c", es = "Caída", en = "Decay" }, { "s", es = "Sostén", en = "Sustain" }, { "r", es = "Soltar", en = "Release" }, { "ancho", es = "Estéreo", en = "Stereo" }, { "vib", es = "Vibrato", en = "Vibrato" } })
    end,
  },
}
