return {
  nombre = "Arpegio", tipo = "herramienta", grupo = "Notas",
  controles = { { "paso", "Cada cuántos pasos", 1, 4, 2 }, { "octavas", "Octavas", 1, 3, 1 } },
  notas = function(lista, c)
    local paso, octavas, acordes, orden, sale = math.floor(c.paso), math.floor(c.octavas), {}, {}, {}
    for _, n in ipairs(lista) do
      if not acordes[n.paso] then acordes[n.paso] = {}; orden[#orden + 1] = n.paso end
      table.insert(acordes[n.paso], n)
    end
    for _, p in ipairs(orden) do
      local a, largo = acordes[p], 0
      table.sort(a, function(x, y) return x.tono < y.tono end)
      for _, n in ipairs(a) do largo = math.max(largo, n.largo) end
      local i, t = 0, 0
      while t < largo do
        local n = a[i % #a + 1]
        local oct = math.floor(i / #a) % octavas
        sale[#sale + 1] = { paso = p + t, largo = paso, tono = n.tono + 12 * oct, fuerza = n.fuerza }
        i, t = i + 1, t + paso
      end
    end
    return sale
  end,
}
