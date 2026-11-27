-- Ácido: un bajo de una sola onda con filtro muy resonante que se cierra en cada nota.

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
  nombre = { es = "Bajo ácido", en = "Acid bass" }, grupo = { es = "Sintetizadores propios", en = "Our synths" }, voces = 4,
  controles = {
    { "forma", { es = "Onda", en = "Wave" }, 0, 1, 0 }, { "corte", { es = "Corte", en = "Cutoff" }, 60, 4000, 400 },
    { "filo", { es = "Filo", en = "Resonance" }, 0, 0.95, 0.85 }, { "abre", { es = "Abre", en = "Env amount" }, 0, 6000, 2200 },
    { "cae", { es = "Caída", en = "Decay" }, 0.03, 2, 0.22 }, { "fuego", { es = "Fuego", en = "Drive" }, 0, 1, 0.35 },
  },
  presets = {
    { nombre = { es = "Clásico", en = "Classic" }, corte = 400, filo = 0.85, abre = 2200, cae = 0.22 },
    { nombre = { es = "Gritón", en = "Screamer" }, corte = 700, filo = 0.93, abre = 4500, cae = 0.35, fuego = 0.8 },
    { nombre = { es = "Hondo", en = "Deep" }, forma = 1, corte = 160, filo = 0.5, abre = 700, cae = 0.4, fuego = 0.15 },
  },
  sonido = function(n, c)
    local x = si(c.forma, pulso(n.hz, 0.5) * 0.8, sierra(n.hz))
    local hz = c.corte + golpe(c.cae) * c.abre * (0.4 + n.fuerza * 0.6)
    x = pasabajos(x, hz, c.filo) * (1 + c.fuego * 5)
    x = curva(x, function(v) return math.tanh(v) end, "suave")
    return x * adsr(0.003, 0.6, 0.6, 0.05) * n.fuerza * (0.4 - c.fuego * 0.15)
  end,
  interfaz = {
    ancho = 460, alto = 220,
    dibujar = function(d, c, m)
      d.acero(0, 0, 460, 220, "#b9bcc2", 10)
      local en = m.idioma == "en"
      d.hueco(20, 14, 150, 30, 5)
      d.texto(32, 29, m.titulo, 20, "#c8ff3d")
      d.onda(206, 14, 84, 28, "#c8ff3d")
      d.led(184, 27, 6, "#c8ff3d", math.min(1, m.nivel * 3))
      d.lista(300, 14, 144, 28, "forma", { en and "Saw" or "Sierra", en and "Square" or "Cuadrada" }, "#c8ff3d")
      d.bandeja(14, 54, 432, 154, "#3a3d42", 8)
      fila(d, m, 62, 112, 84, 30, "#c8ff3d", { { "corte", es = "Corte", en = "Cutoff" }, { "filo", es = "Filo", en = "Reso" }, { "abre", es = "Abre", en = "Env" }, { "cae", es = "Caída", en = "Decay" }, { "fuego", es = "Fuego", en = "Drive" } }, "#b4b7bd")
      -- Cada foco es una parte de lo que suena, de graves a agudos.
      for i = 1, 16 do
        local alto = m.espectro[i * 4] or 0
        d.led(30 + (i - 1) * 26.6, 188, 4.5, i > 12 and "#ff5a4a" or "#c8ff3d", math.max(0, alto * 1.5 - 0.45))
      end
    end,
  },
}
