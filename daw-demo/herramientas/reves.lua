return {
  nombre = "Al revés", tipo = "herramienta", grupo = "Notas",
  notas = function(lista, c)
    for _, n in ipairs(lista) do n.paso = math.max(0, c.largo - n.paso - n.largo) end
    return lista
  end,
}
