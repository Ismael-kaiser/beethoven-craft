local equipo = incluir("equipo")

-- Filtro con patrón: dieciséis pasos dibujan cómo se abre el filtro al ritmo de la canción.
local controles = {
  { "div", { es = "Pasos por negra", en = "Steps per beat" }, 1, 8, 4 }, { "suave", { es = "Suavidad", en = "Smoothness" }, 0.001, 0.2, 0.01 },
  { "tipo", { es = "Tipo", en = "Type" }, 0, 2, 0 }, { "filo", { es = "Resonancia", en = "Resonance" }, 0, 0.95, 0.45 },
  { "empuje", "Saturación", 0, 1, 0 }, { "mezcla", "Mezcla", 0, 1, 1 },
}
for i = 1, 16 do controles[#controles + 1] = { "p" .. i, { es = "Paso " .. i, en = "Step " .. i }, 0, 1, ({ 0.8, 0.35, 0.55, 0.3 })[(i - 1) % 4 + 1] } end
local function dibujo(nombre, f)
  local p = { nombre = nombre }
  for i = 1, 16 do p["p" .. i] = f(i) end
  return p
end
return {
  nombre = { es = "Filtro con patrón", en = "Pattern filter" }, tipo = "efecto", grupo = "Tono",
  controles = controles,
  presets = {
    dibujo({ es = "Subida", en = "Rise" }, function(i) return i / 16 end), dibujo({ es = "Bajada", en = "Fall" }, function(i) return 1 - (i - 1) / 16 end),
    dibujo({ es = "Contratiempo", en = "Offbeat" }, function(i) return i % 4 == 3 and 0.95 or 0.25 end),
  },
  sonido = function(e, c)
    local s = piso(resto(posicion() * piso(c.div + 0.5), 16))
    local v = s * 0
    for i = 1, 16 do v = v + mayor(s, i - 1.5) * mayor(i - 0.5, s) * c["p" .. i] end
    v = mide("paso", seguidor(v, c.suave, c.suave))
    local hz = exp(v * math.log(300)) * 60
    local function es(k) return mayor(c.tipo, k - 0.5) * mayor(k + 0.5, c.tipo) end
    local function f(x)
      local y = pasabajos(x, hz, c.filo) * es(0) + pasaaltos(x, hz, c.filo) * es(1) + pasabanda(x, hz, c.filo) * es(2)
      y = mezclar(y, curva(y * 4, function(u) return math.tanh(u) end, "suave") * 0.6, c.empuje)
      return mezclar(x, y, c.mezcla)
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = {
    ancho = 620, alto = 372,
    dibujar = function(d, c, m)
      d.acero(0, 0, 620, 372, "#54602e", 9)
      d.grabado(34, 25, m.titulo, 20, "#d8f06a")
      d.prender(590, 25, 10, "#d8f06a", "palanca")
      d.lista(330, 14, 210, 26, "tipo", m.idioma == "en" and { "Low", "High", "Band" } or { "Graves", "Agudos", "Banda" }, "#d8f06a")
      d.bandeja(24, 50, 520, 170, "#1c2010", 8)
      for i = 1, 16 do d.tirador(32 + (i - 1) * 32, 60, 24, 150, "p" .. i, (i - 1) % 8 < 4 and "#d8f06a" or "#ffd95e") end
      d.aguja(552, 130, 56, 60, m.medidas.paso or 0, "")
      d.historia(26, 326, 568, 34, "#d8f06a")
      d.bandeja(24, 232, 572, 86, nil, 8)
      equipo.perillas(d, c, m, { x = 30, y = 236, w = 560, claves = { "div", "suave", "filo", "empuje", "mezcla" }, color = "#d8f06a", metal = "#8d8f96", alto = 82, r = 20 })
    end,
  },
}
