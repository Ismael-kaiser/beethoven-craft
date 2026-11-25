local equipo = incluir("equipo")

-- Mueve cualquier perilla con una cuenta tuya: clic derecho en el efecto, «Editar en Lua», y cambia la línea «local x».
-- Vincúlalo: clic derecho en cualquier perilla de un plugin y «Moverlo con…».
return {
  nombre = { es = "Controlador con fórmula", en = "Formula controller" }, tipo = "efecto", grupo = { es = "Controladores", en = "Controllers" },
  controles = { { "a", { es = "a", en = "a" }, 0, 1, 0.5 }, { "b", { es = "b", en = "b" }, 0, 1, 0.5 }, { "c", { es = "c", en = "c" }, 0.05, 8, 1 } },
  presets = { { nombre = { es = "Vaivén", en = "Sway" }, a = 0.5, b = 0.4, c = 0.25 }, { nombre = { es = "Temblor", en = "Tremor" }, a = 0.6, b = 0.2, c = 6 } },
  sonido = function(e, c)
    local a, b, c_ = c.a, c.b, c.c
    local x = a + b * sen(posicion() * c_)
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
      d.texto(206, 130, "x = a + b · sen(pos · c)", 14, "#b8ffb0")
      d.grabado(206, 104, m.idioma == "en" and "Right-click any knob: «Move it with…»" or "Clic derecho en una perilla: «Moverlo con…»", 12, "#f2efe6")
      d.bandeja(24, 156, 472, 82, nil, 8)
      equipo.perillas(d, c, m, { x = 28, y = 160, w = 464, claves = { "a", "b", "c" }, color = "#b8ffb0", metal = "#8d8f96", alto = 78, r = 18 })
    end,
  },
}
