-- Piezas que usa «Voz gruesa». Se traen con incluir("ayuda"): cualquier .lua de la carpeta sirve así.
local ayuda = {}

-- Calienta sin romper: satura suave y deja el nivel parecido.
function ayuda.calentar(x, cuanto)
  return tanh(x * (1 + cuanto * 3)) * (1 / (1 + cuanto))
end

-- Una perilla con su nombre debajo, en el idioma de la app.
function ayuda.perilla(d, x, y, clave, es, en, idioma)
  d.perilla(x, y, 26, clave, "#ffb347")
  d.texto(x, y + 40, idioma == "en" and en or es, 13, "#f3e6cf", "centro")
end

return ayuda
