local equipo = incluir("equipo")

-- Compresor lateral: baja esta pista cuando suena otra (el bombo que hace espacio al bajo).
return {
  nombre = { es = "Compresor lateral", en = "Sidechain compressor" }, tipo = "efecto", grupo = "Dinámica",
  controles = {
    { "pista", { es = "Pista que manda", en = "Trigger track" }, 0, 16, 0 }, { "umbral", "Umbral dB", -60, 0, -30 }, { "cuanto", "Cuánto", 0, 1, 0.8 },
    { "ataque", "Ataque", 0.0005, 0.1, 0.003 }, { "soltar", "Soltar", 0.02, 1.5, 0.18 },
  },
  presets = { { nombre = { es = "Bombeo", en = "Pumping" }, cuanto = 1, soltar = 0.3 }, { nombre = { es = "Espacio para el bombo", en = "Room for the kick" }, cuanto = 0.6, soltar = 0.12 }, { nombre = { es = "Música bajo la voz", en = "Music under voice" }, cuanto = 0.5, ataque = 0.03, soltar = 0.6, umbral = -36 } },
  sonido = function(e, c)
    local n = seguidor(maximo(abs(e.otra_izq), abs(e.otra_der)), c.ataque, c.soltar)
    local g = mide("reduccion", 1 - c.cuanto * minimo(n / (db(c.umbral) * 4), 1))
    return e.izq * g, e.der * g
  end,
  interfaz = equipo.rack{ tinte = "#5c3a52", color = "#ffa8d8", metal = "#8d8f96", grafica = false, aguja = function(c, m) return equipo.baja(m), m.idioma == "en" and "REDUCTION" or "REDUCCIÓN" end,
    arriba = function(d, c, m, x, y, w, h)
      d.grabado(x + 190, y + 26, m.idioma == "en" and "Trigger track" or "Pista que manda", 13, "#f2efe6")
      d.pista(x + 190, y + 42, 190, "pista")
    end, claves = { "umbral", "cuanto", "ataque", "soltar" } },
}
