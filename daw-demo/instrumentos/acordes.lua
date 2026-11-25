-- Acordes: cada tecla suena un acorde de la escala.
local equipo = incluir("equipo")

return {
  nombre = { es = "Acordes con una tecla", en = "One-key chords" }, tipo = "notas", grupo = { es = "Notas en vivo", en = "Live notes" },
  controles = { { "tipo", { es = "Acorde", en = "Chord" }, 0, 4, 0 }, { "rasgueo", { es = "Rasgueo", en = "Strum" }, 0, 0.12, 0.01 }, { "baja", { es = "Con bajo", en = "Add bass" }, 0, 1, 1 } },
  presets = { { nombre = { es = "Mayor", en = "Major" }, tipo = 0 }, { nombre = { es = "Menor séptima", en = "Minor seventh" }, tipo = 3 } },
  vivo = function(v)
    local acordes = { { 0, 4, 7 }, { 0, 3, 7 }, { 0, 4, 7, 11 }, { 0, 3, 7, 10 }, { 0, 5, 7 } }
    local tono, fuerza, i, falta, tipo, baja = 0, 0.0, 0, 0, 1, false
    return {
      nota = function(t, f, c)
        tono, fuerza, i, falta, tipo, baja = t, f, 1, 0, math.floor(c.tipo + 0.5) + 1, c.baja > 0.5
        if baja then toca(t - 12, f, 1.2) end
      end,
      bloque = function(izq, der, n, c, m)
        local a = acordes[tipo]
        falta = falta - n
        while i > 0 and falta <= 0 do
          toca(tono + a[i], fuerza, 1.2)
          i = i < #a and i + 1 or 0
          falta = falta + math.floor(c.rasgueo * v.sr) + 1
          if c.rasgueo > 0 and falta > 0 then break end
        end
      end,
    }
  end,
  interfaz = {
    ancho = 480, alto = 190,
    dibujar = function(d, c, m)
      local en = m.idioma == "en"
      d.acero(0, 0, 480, 190, "#3f6c5a", 10)
      d.grabado(30, 24, m.titulo, 20, "#b4ffd8")
      d.lista(20, 46, 440, 28, "tipo", en and { "Major", "Minor", "Maj 7", "Min 7", "Sus 4" } or { "Mayor", "Menor", "May 7", "Men 7", "Sus 4" }, "#b4ffd8")
      d.bandeja(20, 88, 440, 88, nil, 8)
      equipo.perillas(d, c, m, { x = 24, y = 90, w = 340, claves = { "rasgueo" }, color = "#b4ffd8", metal = "#8d8f96", alto = 84, r = 22 })
      d.palanca(400, 132, "baja", "#b4ffd8")
      d.grabado(400, 166, m.nombres.baja, 11, "#f2efe6", "centro")
    end,
  },
}
