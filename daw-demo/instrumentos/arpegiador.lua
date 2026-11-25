-- Arpegiador: lo que se mantiene apretado sale nota por nota, al ritmo de la canción.
local equipo = incluir("equipo")

return {
  nombre = { es = "Arpegiador", en = "Arpeggiator" }, tipo = "notas", grupo = { es = "Notas en vivo", en = "Live notes" },
  controles = {
    { "paso", { es = "Notas por negra", en = "Notes per beat" }, 1, 8, 4 }, { "orden", { es = "Orden", en = "Order" }, 0, 3, 0 },
    { "octavas", { es = "Octavas", en = "Octaves" }, 1, 4, 2 }, { "largo", { es = "Largo", en = "Length" }, 0.1, 1.5, 0.7 },
  },
  presets = { { nombre = { es = "Sube", en = "Up" }, orden = 0 }, { nombre = { es = "Sube y baja", en = "Up and down" }, orden = 2, octavas = 1 } },
  vivo = function(v)
    local tonos, fuerzas, n, ultimo, k = {}, {}, 0, -1, 0
    for i = 1, 64 do tonos[i], fuerzas[i] = 0, 0.0 end
    return {
      nota = function(tono, fuerza)
        if n >= 64 then return end
        -- En orden, de grave a agudo.
        local i = n
        while i >= 1 and tonos[i] > tono do
          tonos[i + 1], fuerzas[i + 1] = tonos[i], fuerzas[i]
          i = i - 1
        end
        tonos[i + 1], fuerzas[i + 1] = tono, fuerza
        n = n + 1
        if n == 1 then ultimo, k = -1, 0 end
      end,
      suelta = function(tono)
        local j = 0
        for i = 1, n do
          if tonos[i] ~= tono then j = j + 1 tonos[j], fuerzas[j] = tonos[i], fuerzas[i] end
        end
        n = j
      end,
      bloque = function(izq, der, cuantas, c, m)
        if n == 0 then return end
        local paso = math.floor(c.paso + 0.5)
        local ahora = math.floor(m.pos * paso + 1e-6)
        if ahora == ultimo then return end
        ultimo = ahora
        local oct = math.floor(c.octavas + 0.5)
        local total = n * oct
        local orden = math.floor(c.orden + 0.5)
        local i = k % total
        if orden == 1 then i = total - 1 - i
        elseif orden == 2 and total > 1 then
          local q = k % (total * 2 - 2)
          i = q < total and q or total * 2 - 2 - q
        elseif orden == 3 then i = math.random(0, total - 1) end
        k = k + 1
        local j = i % n + 1
        toca(tonos[j] + 12 * (i // n), fuerzas[j], 60 / m.bpm / paso * c.largo)
      end,
    }
  end,
  interfaz = {
    ancho = 480, alto = 190,
    dibujar = function(d, c, m)
      local en = m.idioma == "en"
      d.acero(0, 0, 480, 190, "#4a3f6c", 10)
      d.grabado(30, 24, m.titulo, 20, "#c8b4ff")
      d.lista(20, 46, 440, 28, "orden", en and { "Up", "Down", "Up and down", "Random" } or { "Sube", "Baja", "Sube y baja", "Al azar" }, "#c8b4ff")
      d.bandeja(20, 88, 440, 88, nil, 8)
      equipo.perillas(d, c, m, { x = 24, y = 90, w = 432, claves = { "paso", "octavas", "largo" }, color = "#c8b4ff", metal = "#8d8f96", alto = 84, r = 22 })
    end,
  },
}
