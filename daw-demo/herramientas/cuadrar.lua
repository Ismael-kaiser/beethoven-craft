return {
  nombre = "Cuadrar a la rejilla", tipo = "herramienta", grupo = "Notas",
  controles = { { "rejilla", "Rejilla en pasos", 0.5, 4, 1 } },
  notas = function(lista, c)
    for _, n in ipairs(lista) do n.paso = math.floor(n.paso / c.rejilla + 0.5) * c.rejilla end
    return lista
  end,
}
