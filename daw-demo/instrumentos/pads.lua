-- Batería de 16 pads: cada uno con su audio, volumen, tono, lado y grupo de corte. El primero suena con el do de la
-- segunda octava (nota 36).
local equipo = incluir("equipo")

return {
  muestras = "pads",
  nombre = { es = "Batería de 16 pads", en = "16-pad drum sampler" }, grupo = { es = "Muestras", en = "Samples" }, voces = 24,
  controles = {
    { "tono", "Tono", -24, 24, 0 }, { "corte", "Corte", 100, 18000, 18000 }, { "baja", "Soltar", 0.005, 3, 0.04 }, { "largo", { es = "Largo", en = "Length" }, 0.02, 8, 8 }, { "empuje", "Saturación", 0, 1, 0 },
    { "v1", { es = "Volumen 1", en = "Volume 1" }, 0, 2, 1 }, { "t1", { es = "Tono 1", en = "Pitch 1" }, -24, 24, 0 }, { "l1", { es = "Lado 1", en = "Pan 1" }, -1, 1, 0 }, { "g1", { es = "Corte 1", en = "Choke 1" }, 0, 4, 0 },
    { "v2", { es = "Volumen 2", en = "Volume 2" }, 0, 2, 1 }, { "t2", { es = "Tono 2", en = "Pitch 2" }, -24, 24, 0 }, { "l2", { es = "Lado 2", en = "Pan 2" }, -1, 1, 0 }, { "g2", { es = "Corte 2", en = "Choke 2" }, 0, 4, 0 },
    { "v3", { es = "Volumen 3", en = "Volume 3" }, 0, 2, 1 }, { "t3", { es = "Tono 3", en = "Pitch 3" }, -24, 24, 0 }, { "l3", { es = "Lado 3", en = "Pan 3" }, -1, 1, 0 }, { "g3", { es = "Corte 3", en = "Choke 3" }, 0, 4, 0 },
    { "v4", { es = "Volumen 4", en = "Volume 4" }, 0, 2, 1 }, { "t4", { es = "Tono 4", en = "Pitch 4" }, -24, 24, 0 }, { "l4", { es = "Lado 4", en = "Pan 4" }, -1, 1, 0 }, { "g4", { es = "Corte 4", en = "Choke 4" }, 0, 4, 0 },
    { "v5", { es = "Volumen 5", en = "Volume 5" }, 0, 2, 1 }, { "t5", { es = "Tono 5", en = "Pitch 5" }, -24, 24, 0 }, { "l5", { es = "Lado 5", en = "Pan 5" }, -1, 1, 0 }, { "g5", { es = "Corte 5", en = "Choke 5" }, 0, 4, 0 },
    { "v6", { es = "Volumen 6", en = "Volume 6" }, 0, 2, 1 }, { "t6", { es = "Tono 6", en = "Pitch 6" }, -24, 24, 0 }, { "l6", { es = "Lado 6", en = "Pan 6" }, -1, 1, 0 }, { "g6", { es = "Corte 6", en = "Choke 6" }, 0, 4, 0 },
    { "v7", { es = "Volumen 7", en = "Volume 7" }, 0, 2, 1 }, { "t7", { es = "Tono 7", en = "Pitch 7" }, -24, 24, 0 }, { "l7", { es = "Lado 7", en = "Pan 7" }, -1, 1, 0 }, { "g7", { es = "Corte 7", en = "Choke 7" }, 0, 4, 0 },
    { "v8", { es = "Volumen 8", en = "Volume 8" }, 0, 2, 1 }, { "t8", { es = "Tono 8", en = "Pitch 8" }, -24, 24, 0 }, { "l8", { es = "Lado 8", en = "Pan 8" }, -1, 1, 0 }, { "g8", { es = "Corte 8", en = "Choke 8" }, 0, 4, 0 },
    { "v9", { es = "Volumen 9", en = "Volume 9" }, 0, 2, 1 }, { "t9", { es = "Tono 9", en = "Pitch 9" }, -24, 24, 0 }, { "l9", { es = "Lado 9", en = "Pan 9" }, -1, 1, 0 }, { "g9", { es = "Corte 9", en = "Choke 9" }, 0, 4, 0 },
    { "v10", { es = "Volumen 10", en = "Volume 10" }, 0, 2, 1 }, { "t10", { es = "Tono 10", en = "Pitch 10" }, -24, 24, 0 }, { "l10", { es = "Lado 10", en = "Pan 10" }, -1, 1, 0 }, { "g10", { es = "Corte 10", en = "Choke 10" }, 0, 4, 0 },
    { "v11", { es = "Volumen 11", en = "Volume 11" }, 0, 2, 1 }, { "t11", { es = "Tono 11", en = "Pitch 11" }, -24, 24, 0 }, { "l11", { es = "Lado 11", en = "Pan 11" }, -1, 1, 0 }, { "g11", { es = "Corte 11", en = "Choke 11" }, 0, 4, 0 },
    { "v12", { es = "Volumen 12", en = "Volume 12" }, 0, 2, 1 }, { "t12", { es = "Tono 12", en = "Pitch 12" }, -24, 24, 0 }, { "l12", { es = "Lado 12", en = "Pan 12" }, -1, 1, 0 }, { "g12", { es = "Corte 12", en = "Choke 12" }, 0, 4, 0 },
    { "v13", { es = "Volumen 13", en = "Volume 13" }, 0, 2, 1 }, { "t13", { es = "Tono 13", en = "Pitch 13" }, -24, 24, 0 }, { "l13", { es = "Lado 13", en = "Pan 13" }, -1, 1, 0 }, { "g13", { es = "Corte 13", en = "Choke 13" }, 0, 4, 0 },
    { "v14", { es = "Volumen 14", en = "Volume 14" }, 0, 2, 1 }, { "t14", { es = "Tono 14", en = "Pitch 14" }, -24, 24, 0 }, { "l14", { es = "Lado 14", en = "Pan 14" }, -1, 1, 0 }, { "g14", { es = "Corte 14", en = "Choke 14" }, 0, 4, 0 },
    { "v15", { es = "Volumen 15", en = "Volume 15" }, 0, 2, 1 }, { "t15", { es = "Tono 15", en = "Pitch 15" }, -24, 24, 0 }, { "l15", { es = "Lado 15", en = "Pan 15" }, -1, 1, 0 }, { "g15", { es = "Corte 15", en = "Choke 15" }, 0, 4, 0 },
    { "v16", { es = "Volumen 16", en = "Volume 16" }, 0, 2, 1 }, { "t16", { es = "Tono 16", en = "Pitch 16" }, -24, 24, 0 }, { "l16", { es = "Lado 16", en = "Pan 16" }, -1, 1, 0 }, { "g16", { es = "Corte 16", en = "Choke 16" }, 0, 4, 0 },
  },
  sonido = function(n, c)
    -- Cada pad suena en su tecla sin cambiar de tono.
    local v = n.hz / 261.6256 * 2 ^ (piso(c.tono + 0.5) / 12)
    local e = adsr(0.0005, 0.1, 1, c.baja) * mezclar(golpe(c.largo), 1, mayor(c.largo, 7.9)) * n.fuerza
    local function f(lado)
      local x = pasabajos(muestra(v, lado), c.corte, 0)
      return mezclar(x, curva(x * 4, function(u) return math.tanh(u) end, "suave") * 0.5, c.empuje) * e
    end
    return f("izq"), f("der")
  end,
  interfaz = {
    ancho = 660, alto = 440,
    dibujar = function(d, c, m)
      local en = m.idioma == "en"
      d.acero(0, 0, 660, 440, "#3a3d44", 10)
      d.grabado(30, 24, m.titulo, 21, "#ff8a6a")
      d.bandeja(20, 46, 390, 376, "#1a1b1e", 10)
      d.pads(24, 50, 382, 368, "#ff8a6a")
      local p = m.pad
      d.hueco(428, 50, 212, 40, 6)
      d.texto(534, 70, "Pad " .. p, 20, "#ff8a6a", "centro")
      d.bandeja(424, 104, 220, 178, nil, 8)
      local nombres = en and { "Volume", "Pitch", "Pan", "Choke" } or { "Volumen", "Tono", "Lado", "Corte" }
      for i, k in ipairs({ "v", "t", "l", "g" }) do
        local x, y = 480 + ((i - 1) % 2) * 108, 148 + math.floor((i - 1) / 2) * 86
        d.grabado(x, y - 33, nombres[i], 11.5, "#f2efe6", "centro")
        d.perilla(x, y, 24, k .. p, "#ff8a6a", "#8d8f96")
        local v = c[k .. p]
        d.texto(x, y + 34, k == "g" and (v < 0.5 and "—" or tostring(math.floor(v + 0.5))) or equipo.numero(v), 11, "#ff8a6a", "centro")
      end
      d.bandeja(424, 294, 220, 128, nil, 8)
      equipo.perillas(d, c, m, { x = 428, y = 298, w = 212, claves = { "tono", "corte", "baja", "largo", "empuje" }, por = 3, color = "#ffd95e", alto = 62, r = 14 })
    end,
  },
}
