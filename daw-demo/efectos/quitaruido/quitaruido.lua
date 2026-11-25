local equipo = incluir("equipo")

-- Quita ruido: aprende cómo suena el ruido y lo resta, frecuencia por frecuencia, mientras suena.
return {
  nombre = { es = "Quita ruido", en = "Noise reducer" }, tipo = "efecto", grupo = "Dinámica",
  controles = { { "aprende", { es = "Aprender", en = "Learn" }, 0, 1, 0 }, { "cuanto", "Cuánto", 0, 4, 2 }, { "piso", { es = "Deja pasar dB", en = "Floor dB" }, -40, 0, -24 }, { "mezcla", "Mezcla", 0, 1, 1 } },
  presets = { { nombre = { es = "Suave", en = "Soft" }, cuanto = 1.2, piso = -12 }, { nombre = { es = "Fuerte", en = "Strong" }, cuanto = 3, piso = -36 } },
  medidas = { "hay" },
  vivo = function(v)
    local n = 2048
    local mitad = n // 2 + 1
    local ruido, antes = { {}, {} }, { {}, {} }
    for l = 1, 2 do for i = 1, mitad do ruido[l][i], antes[l][i] = 0.0, 1.0 end end
    local hay = 0.0
    return espectral(n, function(re, im, l, c, m)
      local ru, an = ruido[l], antes[l]
      local piso = 10 ^ (c.piso / 20)
      local aprende = c.aprende > 0.5
      local suma = 0.0
      for i = 1, mitad do
        local a, b = re[i], im[i]
        local x = math.sqrt(a * a + b * b)
        if aprende then ru[i] = ru[i] + (x - ru[i]) * 0.06 end
        suma = suma + ru[i]
        local g = (x - c.cuanto * ru[i]) / (x + 1e-9)
        if g < piso then g = piso end
        -- Sube de golpe y baja despacio: no gorjea.
        local q = an[i] * 0.7
        if g < q then g = q end
        an[i] = g
        g = 1 + (g - 1) * c.mezcla
        re[i], im[i] = a * g, b * g
        if i > 1 and i < mitad then
          local j = n - i + 2
          re[j], im[j] = re[i], -im[i]
        end
      end
      mide("hay", suma > 1e-4 and 1 or 0)
    end)
  end,
  interfaz = equipo.rack{ tinte = "#34506a", color = "#9fd4ff", metal = "#8d8f96", grafica = false,
    arriba = function(d, c, m, x, y, w, h)
      local en = m.idioma == "en"
      d.lista(x, y + 8, 260, 32, "aprende", en and { "Clean", "Learn the noise" } or { "Limpiar", "Aprender el ruido" }, "#9fd4ff")
      d.led(x + 20, y + 76, 7, "#8ae08f", m.medidas.hay or 0)
      d.grabado(x + 40, y + 76, en and "Noise learned" or "Ruido aprendido", 13, "#f2efe6")
      d.grabado(x + 290, y + 24, en and "Play only the noise while learning" or "Al aprender, que solo suene el ruido", 11.5, "#cfe0f0")
    end, claves = { "cuanto", "piso", "mezcla" } },
}
