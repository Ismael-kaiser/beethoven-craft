local equipo = incluir("equipo")

-- Vocoder de 16 bandas: la voz de esta pista moldea a otra pista, o a un sintetizador interno si no eliges ninguna.
return {
  nombre = { es = "Vocoder", en = "Vocoder" }, tipo = "efecto", grupo = "Voz",
  controles = {
    { "pista", { es = "Pista portadora", en = "Carrier track" }, 0, 16, 0 }, { "tono", { es = "Nota interna Hz", en = "Internal note Hz" }, 40, 440, 110 },
    { "acorde", { es = "Acorde", en = "Chord" }, 0, 1, 0.6 }, { "soplo", { es = "Soplo", en = "Breath" }, 0, 1, 0.2 }, { "filo", { es = "Definición", en = "Definition" }, 0.3, 0.95, 0.8 },
    { "ataque", "Ataque", 0.001, 0.1, 0.005 }, { "soltar", "Soltar", 0.01, 0.5, 0.06 }, { "eses", { es = "Consonantes", en = "Consonants" }, 0, 1, 0.4 },
    { "mezcla", "Mezcla", 0, 1, 1 }, { "sale", "Salida dB", -24, 18, 0 },
  },
  presets = { { nombre = { es = "Robot", en = "Robot" }, acorde = 0, tono = 98, filo = 0.88 }, { nombre = { es = "Coro de máquinas", en = "Machine choir" }, acorde = 1, soltar = 0.12 }, { nombre = { es = "Susurro", en = "Whisper" }, soplo = 1, acorde = 0, filo = 0.7 } },
  sonido = function(e, c)
    local voz = (e.izq + e.der) * 0.5
    local dentro = (sierra(c.tono) + (sierra(c.tono * 1.1892) + sierra(c.tono * 1.4983) + sierra(c.tono * 2)) * c.acorde) * 0.4
    local fuera = (e.otra_izq + e.otra_der) * 0.5
    local porta = si(mayor(c.pista, 0.5), fuera, dentro) + ruido() * c.soplo * 0.5
    local a, b = nil, nil
    for i = 1, 16 do
      local hz = 120 * (9500 / 120) ^ ((i - 1) / 15)
      local n = seguidor(abs(pasabanda(voz, hz, c.filo)), c.ataque, c.soltar)
      if i % 4 == 1 and i < 30 then n = mide("b" .. i, n) end
      local s = pasabanda(porta, hz, c.filo) * n * (1 - c.filo * 0.8) * 2
      if i % 2 == 0 then a = a and a + s or s else b = b and b + s or s end
    end
    local eses = pasaaltos(voz, 5000, 0) * c.eses
    -- Las bandas pares cargan a un lado y las nones al otro.
    local izq, der = a * 0.7 + b * 0.4 + eses, a * 0.4 + b * 0.7 + eses
    return mezclar(e.izq, izq * db(c.sale), c.mezcla), mezclar(e.der, der * db(c.sale), c.mezcla)
  end,
  interfaz = equipo.rack{ tinte = "#3a3f6c", color = "#9fb4ff", metal = "#b4b7bd", grafica = false, por = 5,
    arriba = function(d, c, m, x, y, w, h)
      d.hueco(x, y, 250, h, 6)
      for i = 0, 3 do
        local v = math.min(1, (m.medidas["b" .. (i * 4 + 1)] or 0) * 5)
        for j = 0, 9 do d.caja(x + 10 + i * 60, y + h - 16 - j * 9.4, 50, 7, v * 10 > j and (j > 7 and "#ff6a5a" or "#9fb4ff") or "#1a1e33", 2) end
      end
      d.grabado(x + 280, y + 26, m.nombres.pista, 13, "#f2efe6")
      d.pista(x + 280, y + 42, 190, "pista")
    end, claves = { "tono", "acorde", "soplo", "filo", "ataque", "soltar", "eses", "mezcla", "sale" } },
}
