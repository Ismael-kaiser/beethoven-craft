local equipo = incluir("equipo")

-- Filtro de escalera: el circuito de cuatro polos de los sintetizadores de los setenta, calculado muestra por muestra.
return {
  nombre = { es = "Filtro de escalera analógico", en = "Analog ladder filter" }, tipo = "efecto", grupo = "Tono",
  controles = {
    { "corte", "Corte", 20, 18000, 1200 }, { "filo", { es = "Resonancia", en = "Resonance" }, 0, 1.1, 0.4 }, { "empuje", "Saturación", 0, 1, 0.3 },
    { "sigue", { es = "Sigue el nivel", en = "Envelope" }, -1, 1, 0 }, { "negras", { es = "Negras por vuelta", en = "Beats per cycle" }, 0, 8, 0 }, { "hondo", "Hondura", 0, 1, 0.5 },
  },
  presets = { { nombre = { es = "Bajo gordo", en = "Fat bass" }, corte = 380, filo = 0.6, empuje = 0.6 }, { nombre = { es = "Canta solo", en = "Self-oscillating" }, corte = 900, filo = 1.08 }, { nombre = { es = "Guau al tocar", en = "Auto wah" }, corte = 350, filo = 0.7, sigue = 0.8 } },
  vivo = function(v)
    local z = { { 0.0, 0.0, 0.0, 0.0 }, { 0.0, 0.0, 0.0, 0.0 } }
    local env, fase = 0.0, 0.0
    local dos = v.sr * 2
    local function paso(x, e, g, k, d)
      -- Dos pasadas por muestra: la resonancia fuerte no se sale de control.
      local y = e[4]
      for _ = 1, 2 do
        local u = tanh((x - k * y) * d)
        e[1] = e[1] + g * (u - tanh(e[1]))
        e[2] = e[2] + g * (tanh(e[1]) - tanh(e[2]))
        e[3] = e[3] + g * (tanh(e[2]) - tanh(e[3]))
        e[4] = e[4] + g * (tanh(e[3]) - tanh(e[4]))
        y = e[4]
      end
      return y
    end
    return function(l, r, c, m)
      local a = math.abs(l) + math.abs(r)
      env = env + (a - env) * (a > env and 0.01 or 0.0006)
      local oct = c.sigue * env * 5
      if c.negras > 0.05 then
        fase = (fase + m.bpm / 60 / c.negras / v.sr) % 1
        oct = oct + math.sin(fase * 6.2832) * c.hondo * 2
      end
      local f = math.min(math.max(c.corte * 2 ^ oct, 20), 18000)
      local g = 1 - math.exp(-6.2832 * f / dos)
      local d = 1 + c.empuje * 4
      return paso(l, z[1], g, c.filo * 4, d) * (1 + c.filo), paso(r, z[2], g, c.filo * 4, d) * (1 + c.filo)
    end
  end,
  interfaz = equipo.rack{ tinte = "#2c2c30", color = "#ff9a52", metal = "#b4b7bd", grafica = false, aguja = function(c, m) return equipo.vu(m.nivel), "VU" end, claves = { "corte", "filo", "empuje", "sigue", "negras", "hondo" } },
}
