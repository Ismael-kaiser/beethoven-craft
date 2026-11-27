-- Síntesis FM de seis operadores: una matriz decide quién mueve a quién.
local equipo = incluir("equipo")

return {
  nombre = { es = "Sinte FM de 6 operadores", en = "6-operator FM synth" }, grupo = { es = "Sintetizadores propios", en = "Our synths" }, voces = 12,
  controles = {
    { "o1", { es = "Sale 1", en = "Out 1" }, 0, 1, 0.8 },
    { "o2", { es = "Sale 2", en = "Out 2" }, 0, 1, 0 },
    { "o3", { es = "Sale 3", en = "Out 3" }, 0, 1, 0 },
    { "o4", { es = "Sale 4", en = "Out 4" }, 0, 1, 0.35 },
    { "o5", { es = "Sale 5", en = "Out 5" }, 0, 1, 0 },
    { "o6", { es = "Sale 6", en = "Out 6" }, 0, 1, 0 },
    { "r1", { es = "Razón 1", en = "Ratio 1" }, 0.25, 16, 1 },
    { "r2", { es = "Razón 2", en = "Ratio 2" }, 0.25, 16, 1 },
    { "r3", { es = "Razón 3", en = "Ratio 3" }, 0.25, 16, 2 },
    { "r4", { es = "Razón 4", en = "Ratio 4" }, 0.25, 16, 1 },
    { "r5", { es = "Razón 5", en = "Ratio 5" }, 0.25, 16, 14 },
    { "r6", { es = "Razón 6", en = "Ratio 6" }, 0.25, 16, 1 },
    { "d1", { es = "Caída 1", en = "Decay 1" }, 0.02, 12, 8 },
    { "d2", { es = "Caída 2", en = "Decay 2" }, 0.02, 12, 1.2 },
    { "d3", { es = "Caída 3", en = "Decay 3" }, 0.02, 12, 2 },
    { "d4", { es = "Caída 4", en = "Decay 4" }, 0.02, 12, 8 },
    { "d5", { es = "Caída 5", en = "Decay 5" }, 0.02, 12, 0.1 },
    { "d6", { es = "Caída 6", en = "Decay 6" }, 0.02, 12, 1 },
    { "m12", { es = "2 mueve 1", en = "2 moves 1" }, 0, 1.5, 0.35 },
    { "m13", { es = "3 mueve 1", en = "3 moves 1" }, 0, 1.5, 0 },
    { "m14", { es = "4 mueve 1", en = "4 moves 1" }, 0, 1.5, 0 },
    { "m15", { es = "5 mueve 1", en = "5 moves 1" }, 0, 1.5, 0 },
    { "m16", { es = "6 mueve 1", en = "6 moves 1" }, 0, 1.5, 0 },
    { "m23", { es = "3 mueve 2", en = "3 moves 2" }, 0, 1.5, 0 },
    { "m24", { es = "4 mueve 2", en = "4 moves 2" }, 0, 1.5, 0 },
    { "m25", { es = "5 mueve 2", en = "5 moves 2" }, 0, 1.5, 0 },
    { "m26", { es = "6 mueve 2", en = "6 moves 2" }, 0, 1.5, 0 },
    { "m34", { es = "4 mueve 3", en = "4 moves 3" }, 0, 1.5, 0 },
    { "m35", { es = "5 mueve 3", en = "5 moves 3" }, 0, 1.5, 0 },
    { "m36", { es = "6 mueve 3", en = "6 moves 3" }, 0, 1.5, 0 },
    { "m45", { es = "5 mueve 4", en = "5 moves 4" }, 0, 1.5, 0.12 },
    { "m46", { es = "6 mueve 4", en = "6 moves 4" }, 0, 1.5, 0 },
    { "m56", { es = "6 mueve 5", en = "6 moves 5" }, 0, 1.5, 0 },
    { "a", { es = "Ataque", en = "Attack" }, 0.001, 2, 0.002 }, { "c", { es = "Caída", en = "Decay" }, 0.05, 6, 1.8 },
    { "s", { es = "Sostén", en = "Sustain" }, 0, 1, 0 }, { "r", { es = "Soltar", en = "Release" }, 0.005, 4, 0.3 },
    { "vib", { es = "Vibrato", en = "Vibrato" }, 0, 1, 0 }, { "corte", { es = "Corte", en = "Cutoff" }, 80, 18000, 18000 }, { "filo", { es = "Resonancia", en = "Resonance" }, 0, 0.9, 0 },
  },
  presets = {
    { nombre = { es = "Piano eléctrico", en = "Electric piano" }, m12 = 0.35, d2 = 1.2, m45 = 0.12, r5 = 14, d5 = 0.1, c = 2.2 },
    { nombre = { es = "Campana", en = "Bell" }, o4 = 0.5, m12 = 0.7, r2 = 3.5, d2 = 3, m45 = 0.4, r4 = 2, r5 = 7.07, d5 = 1.5, c = 4, r = 1.5 },
    { nombre = { es = "Bajo de goma", en = "Rubber bass" }, o4 = 0, m12 = 0.7, r2 = 0.5, d2 = 0.2, m23 = 0.4, r3 = 2, d3 = 0.1, c = 0.6, s = 0.4, r = 0.08 },
    { nombre = { es = "Órgano de vidrio", en = "Glass organ" }, o4 = 0.4, m12 = 0.2, r2 = 2, d2 = 12, m45 = 0.15, r4 = 2, r5 = 4, d5 = 12, a = 0.02, s = 1, r = 0.15 },
    { nombre = { es = "Metal roto", en = "Broken metal" }, m12 = 0.9, r2 = 1.41, m23 = 0.8, r3 = 2.76, m34 = 0.5, r4 = 5.4, o4 = 0, d2 = 2, d3 = 1, d4 = 0.4, c = 3 },
  },
  sonido = function(n, c)
    local hz = n.hz * (1 + seno(5.5) * c.vib * 0.02)
    local viva = 0.35 + n.fuerza * 0.65
    local op, sale = {}, nil
    for i = 6, 1, -1 do
      local fase = nil
      for j = i + 1, 6 do
        local x = op[j] * c["m" .. i .. j]
        fase = fase and fase + x or x
      end
      op[i] = seno(hz * c["r" .. i], fase) * golpe(c["d" .. i]) * (i == 1 and 1 or viva)
      local x = op[i] * c["o" .. i]
      sale = sale and sale + x or x
    end
    return pasabajos(sale, c.corte, c.filo) * adsr(c.a, c.c, c.s, c.r) * n.fuerza * 0.3
  end,
  interfaz = {
    ancho = 700, alto = 500,
    dibujar = function(d, c, m)
      d.acero(0, 0, 700, 500, "#3f6a66", 10)
      d.grabado(30, 24, m.titulo, 21, "#7ff2d8")
      local en = m.idioma == "en"
      d.bandeja(18, 46, 520, 296, nil, 8)
      d.grabado(160, 58, en and "Who moves whom" or "Quién mueve a quién", 11, "#7ff2d8", "centro")
      for i, n in ipairs({ en and "Ratio" or "Razón", en and "Decay" or "Caída", en and "Out" or "Sale" }) do d.grabado(384 + (i - 1) * 56, 58, n, 11, "#ffd95e", "centro") end
      for i = 1, 6 do
        local y = 86 + (i - 1) * 45
        d.grabado(32, y, tostring(i), 16, "#f2efe6", "centro")
        for j = 2, 6 do
          local x = 70 + (j - 2) * 54
          if j > i then
            d.perilla(x, y, 14, "m" .. i .. j, "#7ff2d8")
            d.texto(x, y + 21, tostring(j), 9, "#7ff2d8", "centro")
          else
            d.circulo(x, y, 3, "#14201f")
          end
        end
        d.perilla(384, y, 15, "r" .. i, "#ffd95e")
        d.texto(384, y + 22, equipo.numero(c["r" .. i]), 9, "#ffd95e", "centro")
        d.perilla(440, y, 15, "d" .. i, "#ffd95e")
        d.perilla(496, y, 15, "o" .. i, "#8ae08f", "#b4b7bd")
      end
      d.onda(552, 50, 130, 130, "#7ff2d8")
      d.bandeja(552, 194, 130, 148, nil, 8)
      equipo.perillas(d, c, m, { x = 554, y = 200, w = 126, claves = { "vib", "corte", "filo" }, por = 2, color = "#8fd0ff", metal = "#8d8f96", alto = 70, r = 17 })
      d.bandeja(18, 354, 250, 92, nil, 8)
      equipo.perillas(d, c, m, { x = 22, y = 358, w = 242, claves = { "a", "c", "s", "r" }, color = "#8ae08f", metal = "#8d8f96", alto = 84, r = 20 })
      d.teclado(284, 360, 396, 110, 48, 25)
    end,
  },
}
