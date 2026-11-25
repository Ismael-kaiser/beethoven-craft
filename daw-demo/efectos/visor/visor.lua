-- Visor: no cambia el sonido; muestra la onda y el espectro de su pista.
return {
  nombre = { es = "Analizador", en = "Analyzer" }, tipo = "efecto", grupo = { es = "Medir", en = "Meters" },
  controles = { { "vista", { es = "Vista", en = "View" }, 0, 1, 0 } },
  presets = {
    { nombre = { es = "Onda", en = "Waveform" }, vista = 0 },
    { nombre = { es = "Espectro", en = "Spectrum" }, vista = 1 },
  },
  sonido = function(e, c)
    return e.izq, e.der
  end,
  interfaz = {
    ancho = 460, alto = 250,
    dibujar = function(d, c, m)
      local en = m.idioma == "en"
      local espectro = c.vista > 0.5
      d.acero(0, 0, 460, 250, "#45484f", 10)
      d.hueco(12, 40, 436, 198, 6)
      d.lista(26, 9, 200, 24, "vista", { en and "Waveform" or "Onda", en and "Spectrum" or "Espectro" }, "#ffd95e")
      d.texto(434, 21, string.format("%.1f dB", 20 * math.log(math.max(m.nivel, 0.00001), 10)), 14, "#ffd95e", "derecha")
      if espectro then
        for i = 1, #m.espectro do
          local h = m.espectro[i] * 190
          d.degradado(16 + (i - 1) * 6.7, 234 - h, 5, h, "#ffd95e", "#c0522c", 1)
        end
      elseif #m.onda > 1 then
        local v = {}
        for i = 1, #m.onda do
          v[#v + 1] = 14 + (i - 1) / (#m.onda - 1) * 432
          v[#v + 1] = 139 - math.max(-1, math.min(1, m.onda[i])) * 94
        end
        d.linea(v, "#8ae08f", 1.5)
      end
      if #m.espectro == 0 then
        d.texto(230, 139, en and "Play something to see it" or "Toca algo para verlo", 15, "#6c6e75", "centro")
      end
    end,
  },
}
