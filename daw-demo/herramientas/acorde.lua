return {
  nombre = "Hacer acordes", tipo = "herramienta", grupo = "Notas",
  controles = { { "menor", "0 mayor · 1 menor", 0, 1, 0 }, { "septima", "Séptima (0 o 1)", 0, 1, 0 } },
  notas = function(lista, c)
    local menor, sep = c.menor >= 0.5, c.septima >= 0.5
    local t, sale = { 0, menor and 3 or 4, 7 }, {}
    if sep then t[4] = menor and 10 or 11 end
    for _, n in ipairs(lista) do
      for _, d in ipairs(t) do
        sale[#sale + 1] = { paso = n.paso, largo = n.largo, tono = n.tono + d, fuerza = n.fuerza }
      end
    end
    return sale
  end,
}
