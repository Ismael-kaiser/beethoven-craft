local equipo = incluir("equipo")

-- Matriz estéreo: cuánto de cada lado va a cada lado, con retardo y fase.
return {
  nombre = { es = "Matriz estéreo", en = "Stereo shaper" }, tipo = "efecto", grupo = { es = "Estéreo", en = "Stereo" },
  controles = {
    { "ii", { es = "Izq a izq", en = "Left to left" }, -1, 1, 1 }, { "di", { es = "Der a izq", en = "Right to left" }, -1, 1, 0 },
    { "id", { es = "Izq a der", en = "Left to right" }, -1, 1, 0 }, { "dd", { es = "Der a der", en = "Right to right" }, -1, 1, 1 },
    { "seg", { es = "Retardo del cruce", en = "Cross delay" }, 0, 0.04, 0 }, { "hz", { es = "Fase del cruce Hz", en = "Cross phase Hz" }, 20, 8000, 20 },
  },
  presets = {
    { nombre = { es = "Cambiar lados", en = "Swap sides" }, ii = 0, di = 1, id = 1, dd = 0 }, { nombre = "Mono", ii = 0.5, di = 0.5, id = 0.5, dd = 0.5 },
    { nombre = { es = "Quitar el centro", en = "Remove the center" }, ii = 1, di = -1, id = -1, dd = 1 }, { nombre = { es = "Ancho con retardo", en = "Delay width" }, di = 0.4, id = -0.4, seg = 0.018 },
  },
  sonido = function(e, c)
    local function cruza(x) return pasatodo(retardo(x, c.seg, 0, 0.05), c.hz) end
    return e.izq * c.ii + cruza(e.der) * c.di, e.der * c.dd + cruza(e.izq) * c.id
  end,
  interfaz = {
    ancho = 560, alto = 260,
    dibujar = function(d, c, m)
      d.acero(0, 0, 560, 260, "#4c5a74", 9)
      d.grabado(34, 25, m.titulo, 20, "#a8d0ff")
      d.prender(526, 25, 10, "#a8d0ff", "palanca")
      d.hueco(40, 50, 180, 180, 8)
      -- Los cuatro caminos: más gruesos cuanto más pasa; rojos si van volteados.
      for _, p in ipairs({ { "ii", 60, 90, 200, 90 }, { "di", 60, 190, 200, 90 }, { "id", 60, 90, 200, 190 }, { "dd", 60, 190, 200, 190 } }) do
        local v = c[p[1]]
        if math.abs(v) > 0.01 then d.linea({ p[2], p[3], p[4], p[5] }, v > 0 and "#a8d0ff" or "#ff6a5a", 1 + math.abs(v) * 5) end
      end
      for i, n in ipairs({ m.idioma == "en" and "L" or "I", m.idioma == "en" and "R" or "D" }) do
        local y = 90 + (i - 1) * 100
        d.circulo(60, y, 14, "#1c2536") d.texto(60, y, n, 15, "#e6f0ff", "centro")
        d.circulo(200, y, 14, "#1c2536") d.texto(200, y, n, 15, "#e6f0ff", "centro")
      end
      d.bandeja(240, 50, 240, 180, nil, 8)
      equipo.perillas(d, c, m, { x = 244, y = 56, w = 232, claves = { "ii", "di", "seg", "id", "dd", "hz" }, por = 3, color = "#a8d0ff", metal = "#8d8f96", alto = 86, r = 22 })
    end,
  },
}
