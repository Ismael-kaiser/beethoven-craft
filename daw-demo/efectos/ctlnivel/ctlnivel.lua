local equipo = incluir("equipo")

-- Sigue qué tan fuerte suena la pista y con eso mueve cualquier perilla.
-- Vincúlalo: clic derecho en cualquier perilla de un plugin y «Moverlo con…».
return {
  nombre = { es = "Controlador por nivel", en = "Peak controller" }, tipo = "efecto", grupo = { es = "Controladores", en = "Controllers" },
  controles = { { "base", { es = "Base", en = "Base" }, 0, 1, 0 }, { "cuanto", { es = "Cuánto", en = "Amount" }, -4, 4, 1.5 }, { "ataque", "Ataque", 0.0005, 0.3, 0.005 }, { "soltar", "Soltar", 0.01, 2, 0.2 }, { "curva", { es = "Curva", en = "Tension" }, 0.25, 4, 1 } },
  presets = { { nombre = { es = "Sube con el golpe", en = "Up with the hit" }, cuanto = 2 }, { nombre = { es = "Baja con el golpe", en = "Down with the hit" }, base = 1, cuanto = -2 } },
  sonido = function(e, c)
    local n = seguidor(maximo(abs(e.izq), abs(e.der)), c.ataque, c.soltar)
    local x = c.base + n ^ c.curva * c.cuanto
    local v = mide("control", minimo(maximo(x, 0), 1))
    -- El sonido pasa igual; el valor se queda en la cuenta sin cambiar nada.
    return e.izq + v * 0, e.der
  end,
  interfaz = {
    ancho = 520, alto = 250,
    dibujar = function(d, c, m)
      d.acero(0, 0, 520, 250, "#4a4a58", 9)
      d.grabado(34, 25, m.titulo, 20, "#b8ffb0")
      d.prender(480, 60, 12, "#b8ffb0", "palanca")
      local v = m.medidas.control or 0
      d.aguja(34, 50, 150, 92, v, string.format("%.0f %%", v * 100))
      for i = 0, 9 do d.led(210 + i * 24, 70, 6, "#b8ffb0", v * 10 > i and 1 or 0) end

      d.grabado(206, 104, m.idioma == "en" and "Right-click any knob: «Move it with…»" or "Clic derecho en una perilla: «Moverlo con…»", 12, "#f2efe6")
      d.bandeja(24, 156, 472, 82, nil, 8)
      equipo.perillas(d, c, m, { x = 28, y = 160, w = 464, claves = { "base", "cuanto", "ataque", "soltar", "curva" }, color = "#b8ffb0", metal = "#8d8f96", alto = 78, r = 18 })
    end,
  },
}
