-- Voz gruesa: más cuerpo y grave en la voz, un poco de calor y un doblado corto que la ensancha.
-- Es el ejemplo de un efecto en su carpeta: trae su logo (logo.png), su fondo (fondo.png) y otro guion (ayuda.lua).
local ayuda = incluir("ayuda")

return {
  nombre = { es = "Voz gruesa", en = "Thick voice" }, tipo = "efecto", grupo = { es = "Voz", en = "Voice" },
  logo = "logo.png",
  controles = {
    { "cuerpo", { es = "Cuerpo", en = "Body" }, 0, 12, 6 },
    { "grave", { es = "Grave", en = "Low end" }, 0, 12, 4 },
    { "calor", { es = "Calor", en = "Warmth" }, 0, 1, 0.3 },
    { "doble", { es = "Doblado", en = "Double" }, 0, 1, 0.35 },
    { "mezcla", { es = "Mezcla", en = "Mix" }, 0, 1, 1 },
  },

  presets = {
    { nombre = "Un poco más de cuerpo", cuerpo = 3, grave = 2, calor = 0.15, doble = 0.1 },
    { nombre = "Locutor", cuerpo = 8, grave = 6, calor = 0.35, doble = 0 },
  },
  sonido = function(e, c)
    local function lado(x, seg)
      local g = campana(x, 230, c.cuerpo, 1.2)
      g = graves(g, 110, c.grave, 0.7)
      g = ayuda.calentar(g, c.calor)
      -- El doblado: la misma voz unos milisegundos después, distinta en cada lado.
      local d = retardo(g, seg, 0, 0.05)
      return mezclar(x, g + d * c.doble * 0.6, c.mezcla)
    end
    return lado(e.izq, 0.017), lado(e.der, 0.023)
  end,

  interfaz = {
    ancho = 360, alto = 200,
    dibujar = function(d, c, m)
      d.imagen(0, 0, 360, 200, "fondo.png")
      -- El logo late un poco con el sonido.
      local l = 46 + m.nivel * 10
      d.imagen(36 - l / 2, 34 - l / 2, l, l, "logo.png")
      d.texto(68, 34, m.idioma == "en" and "THICK VOICE" or "VOZ GRUESA", 22, "#ffd95e")
      d.onda(196, 22, 132, 26, "#ffd95e")
      d.tornillo(344, 14, 4)
      ayuda.perilla(d, 42, 128, "cuerpo", "Cuerpo", "Body", m.idioma)
      ayuda.perilla(d, 111, 128, "grave", "Grave", "Low end", m.idioma)
      ayuda.perilla(d, 180, 128, "calor", "Calor", "Warmth", m.idioma)
      ayuda.perilla(d, 249, 128, "doble", "Doblado", "Double", m.idioma)
      ayuda.perilla(d, 318, 128, "mezcla", "Mezcla", "Mix", m.idioma)
    end,
  },
}
