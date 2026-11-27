-- Sinte mono analógico: un solo sonido con deslizamiento, filtro de escalera y rueda de tono, calculado muestra por muestra.
local equipo = incluir("equipo")

return {
  nombre = { es = "Sinte mono analógico", en = "Mono analog synth" }, grupo = { es = "Sintetizadores propios", en = "Our synths" },
  controles = {
    { "onda", { es = "Onda", en = "Wave" }, 0, 1, 0 }, { "ancho", { es = "Ancho", en = "Width" }, 0.05, 0.95, 0.5 }, { "sub", { es = "Grave", en = "Sub" }, 0, 1, 0.4 }, { "desliza", { es = "Desliza", en = "Glide" }, 0, 0.5, 0.06 },
    { "corte", { es = "Corte", en = "Cutoff" }, 30, 12000, 500 }, { "filo", { es = "Resonancia", en = "Resonance" }, 0, 1.05, 0.5 }, { "abre", { es = "Abre", en = "Env amount" }, 0, 6, 3 }, { "cierra", { es = "Cierra", en = "Env decay" }, 0.02, 3, 0.35 }, { "empuje", "Saturación", 0, 1, 0.3 },
    { "a", { es = "Ataque", en = "Attack" }, 0.001, 2, 0.003 }, { "r", { es = "Soltar", en = "Release" }, 0.005, 3, 0.12 }, { "vib", { es = "Vibrato (mando 1)", en = "Vibrato (mod wheel)" }, 0, 1, 0.5 },
  },
  presets = { { nombre = { es = "Bajo redondo", en = "Round bass" }, corte = 250, abre = 2.5 }, { nombre = { es = "Solista", en = "Lead" }, corte = 1500, sub = 0.1, desliza = 0.12, r = 0.3 }, { nombre = { es = "Chicle", en = "Bubblegum" }, filo = 0.95, cierra = 0.15, abre = 4 } },
  vivo = function(v)
    local apretadas, cuantas = {}, 0
    for i = 1, 32 do apretadas[i] = 0 end
    local f, meta, fase, fase2 = 110.0, 110.0, 0.0, 0.0
    local amp, env, puerta, fuerza, rueda, mod, lfo = 0.0, 0.0, false, 0.0, 0.0, 0.0, 0.0
    local z1, z2, z3, z4 = 0.0, 0.0, 0.0, 0.0
    local sr = v.sr
    local function pulir(t, dt)
      if t < dt then t = t / dt return t + t - t * t - 1 end
      if t > 1 - dt then t = (t - 1) / dt return t * t + t + t + 1 end
      return 0.0
    end
    return {
      nota = function(tono, fz)
        if cuantas < 32 then cuantas = cuantas + 1 apretadas[cuantas] = tono end
        meta = hz(tono)
        -- Ligado si ya había otra apretada: desliza sin volver a atacar.
        if cuantas == 1 then
          env = 1.0
          if amp < 0.001 then f = meta end
        end
        fuerza, puerta = fz, true
      end,
      suelta = function(tono)
        local j = 0
        for i = 1, cuantas do
          if apretadas[i] ~= tono then j = j + 1 apretadas[j] = apretadas[i] end
        end
        cuantas = j
        if j == 0 then puerta = false else meta = hz(apretadas[j]) end
      end,
      rueda = function(x) rueda = x end,
      mando = function(n, x) if n == 1 then mod = x end end,
      muestra = function(_, _, c)
        f = f + (meta - f) * (c.desliza < 0.002 and 1 or 1 / (c.desliza * 0.35 * sr))
        lfo = (lfo + 5.5 / sr) % 1
        local hz_ = f * 2 ^ (rueda / 6 + math.sin(lfo * 6.2832) * mod * c.vib * 0.04)
        local dt = math.min(hz_ / sr, 0.45)
        fase = (fase + dt) % 1
        fase2 = (fase2 + dt * 0.5) % 1
        local o
        if c.onda < 0.5 then
          o = 2 * fase - 1 - pulir(fase, dt)
        else
          o = (fase < c.ancho and 1 or -1) + pulir(fase, dt) - pulir((fase - c.ancho) % 1, dt)
        end
        o = o * 0.5 + (fase2 < 0.5 and 1 or -1) * c.sub * 0.35
        amp = puerta and math.min(1, amp + 1 / (c.a * sr)) or amp * math.exp(-4.6 / (c.r * sr))
        env = env * math.exp(-4.6 / (c.cierra * sr))
        local g = 1 - math.exp(-6.2832 * math.min(c.corte * 2 ^ (env * c.abre), 16000) / (sr * 2))
        local k, d = c.filo * 4, 1 + c.empuje * 3
        for _ = 1, 2 do
          local u = tanh(o * d - k * z4)
          z1 = z1 + g * (u - tanh(z1))
          z2 = z2 + g * (tanh(z1) - tanh(z2))
          z3 = z3 + g * (tanh(z2) - tanh(z3))
          z4 = z4 + g * (tanh(z3) - tanh(z4))
        end
        return z4 * amp * fuerza * (0.6 + c.filo * 0.5)
      end,
    }
  end,
  interfaz = {
    ancho = 640, alto = 340,
    dibujar = function(d, c, m)
      local en = m.idioma == "en"
      d.acero(0, 0, 640, 340, "#3a3026", 10)
      d.grabado(30, 24, m.titulo, 21, "#ffb060")
      d.lista(400, 12, 216, 26, "onda", en and { "Saw", "Pulse" } or { "Sierra", "Pulso" }, "#ffb060")
      d.bandeja(20, 48, 260, 98, nil, 8)
      equipo.perillas(d, c, m, { x = 24, y = 52, w = 252, claves = { "ancho", "sub", "desliza" }, color = "#ffb060", metal = "#c9b58a", alto = 92, r = 23 })
      d.bandeja(292, 48, 328, 98, nil, 8)
      equipo.perillas(d, c, m, { x = 296, y = 52, w = 320, claves = { "corte", "filo", "abre", "cierra", "empuje" }, color = "#8fd0ff", metal = "#8d8f96", alto = 92, r = 19 })
      d.bandeja(20, 158, 260, 88, nil, 8)
      equipo.perillas(d, c, m, { x = 24, y = 160, w = 252, claves = { "a", "r", "vib" }, color = "#8ae08f", metal = "#8d8f96", alto = 84, r = 20 })
      d.onda(296, 162, 320, 80, "#ffb060")
      d.teclado(24, 260, 592, 66, 36, 37)
    end,
  },
}
