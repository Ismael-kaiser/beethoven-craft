-- Ventanas de aparato para instrumentos y efectos: rack, pedal, grabadora de cinta y de bulbos.
-- Se traen con incluir("equipo"); cada una recibe qué controles lleva y de qué color es.
local E = {}

function E.numero(v)
  local a = math.abs(v)
  if a >= 10000 then return string.format("%.0fk", v / 1000) end
  if a >= 1000 then return string.format("%.1fk", v / 1000) end
  if a >= 100 then return string.format("%.0f", v) end
  if a >= 10 then return string.format("%.1f", v) end
  return string.format("%.2f", v)
end

-- Del nivel (0 a 1) a la aguja de un medidor de volumen.
function E.vu(nivel)
  local db = 20 * math.log(math.max(nivel, 0.00001), 10)
  return math.max(0, math.min(1, (db + 36) / 39))
end

-- Cuánto baja un compresor: la aguja descansa a la derecha y cae.
function E.baja(m, cual)
  local g = (m.activo and m.medidas[cual or "reduccion"]) or 1
  -- En cero es que todavía no ha sonado.
  if g <= 0 then g = 1 end
  local db = -20 * math.log(math.max(g, 0.001), 10)
  return 1 - math.max(0, math.min(1, db / 20))
end

-- Perillas en fila, con su nombre y su valor.
function E.perillas(d, c, m, o)
  local n = #o.claves
  local por = o.por or n
  local paso = o.w / por
  local r = o.r or math.min(26, paso * 0.32)
  local alto = o.alto or (r * 2 + 36)
  for i, k in ipairs(o.claves) do
    local f = math.floor((i - 1) / por)
    local en_fila = math.min(por, n - f * por)
    local x = o.x + (o.w - en_fila * paso) / 2 + paso * ((i - 1) % por + 0.5)
    local y = o.y + f * alto + 13 + r
    d.grabado(x, y - r - 7, m.nombres[k] or k, math.min(11.5, paso * 0.16 + 1), o.tinta or "#f2efe6", "centro")
    d.perilla(x, y, r, k, o.color, o.metal)
    d.texto(x, y + r + 8, E.numero(c[k]), 10.5, o.color, "centro")
  end
  return math.ceil(n / por) * alto
end

local function filas(n, por) return math.ceil(n / por) end

local function frente(d, m, o, w, h)
  d.acero(0, 0, w, h, o.tinte, 9)
  if o.asas ~= false then
    d.asa(13, 26, 17, h - 52)
    d.asa(w - 30, 26, 17, h - 52)
  end
  d.grabado(48, 25, m.titulo, 20, o.letras or o.color)
  d.prender(w - 60, 60, 12, o.foco or o.color, "palanca")
end

-- Rack de estudio: gráfica, aguja y perillas.
function E.rack(o)
  local por = o.por or math.min(7, #o.claves)
  local w = o.ancho or 640
  local arriba = (o.grafica == false and not o.aguja and not o.arriba) and 92 or 172
  local h = arriba + filas(#o.claves, por) * 92 + 20
  return {
    ancho = w, alto = h,
    dibujar = function(d, c, m)
      frente(d, m, o, w, h)
      local x = 48
      local libre = w - 48 - 92
      if o.aguja then libre = libre - 168 end
      if o.grafica ~= false then
        d.grafica(x, 46, libre, 108, o.que)
        x = x + libre + 16
      end
      if o.aguja then
        local v, nombre = o.aguja(c, m)
        d.aguja(x + 2, 50, 146, 100, v, nombre)
      end
      if o.arriba then o.arriba(d, c, m, 48, 46, w - 48 - 92, 108) end
      d.bandeja(44, arriba, w - 88, h - arriba - 16, o.charola, 8)
      E.perillas(d, c, m, { x = 50, y = arriba + 8, w = w - 100, claves = o.claves, por = por, color = o.color, metal = o.metal, alto = 92, r = 23 })
    end,
  }
end

-- Pedal de guitarra: pantalla chica, perillas y el pisotón.
function E.pedal(o)
  local por = o.por or (#o.claves == 4 and 2 or math.min(3, #o.claves))
  local w = o.ancho or math.max(250, por * 84 + 30)
  local f = filas(#o.claves, por)
  local h = 112 + f * 90 + 118
  return {
    ancho = w, alto = h,
    dibujar = function(d, c, m)
      d.acero(0, 0, w, h, o.tinte, 16)
      d.grafica(24, 26, w - 48, 70, o.que)
      d.bandeja(16, 112, w - 32, f * 90 + 2, o.charola or "#1d1e21", 9)
      E.perillas(d, c, m, { x = 18, y = 116, w = w - 36, claves = o.claves, por = por, color = o.color, metal = o.metal, alto = 90, r = 23 })
      d.grabado(w / 2, h - 96, m.titulo, o.letra or 21, o.letras or "#fff6e2", "centro")
      d.prender(w / 2, h - 36, 17, o.foco or "#ff4a3a")
    end,
  }
end

-- Grabadora: dos carretes que giran al sonar y su aguja.
function E.cinta(o)
  local por = o.por or math.min(6, #o.claves)
  local w = 640
  local h = 260 + filas(#o.claves, por) * 92 + 20
  return {
    ancho = w, alto = h,
    dibujar = function(d, c, m)
      frente(d, m, o, w, h)
      d.bandeja(48, 44, w - 140, 196, "#17181a", 10)
      local giro = (m.tocando and m.activo) and m.tiempo * (o.vueltas and o.vueltas(c) or 0.5) or 0
      d.carrete(134, 136, 78, giro, 0.8)
      d.carrete(w - 178, 136, 78, giro * 1.25, 0.45)
      -- La cinta pasa por la cabeza.
      local cabeza = w / 2 - 22
      d.linea({ 100, 206, cabeza - 34, 228, cabeza + 34, 228, w - 146, 206 }, "#7a4c30", 3)
      d.acero(cabeza - 30, 206, 60, 28, "#8a8d92", 4)
      d.aguja(cabeza - 56, 66, 112, 78, E.vu(m.nivel), "VU")
      for i = 0, 5 do d.led(cabeza - 40 + i * 16, 170, 3.6, i > 3 and "#ff5a4a" or o.color, m.nivel * 6 > i and 1 or 0) end
      d.bandeja(44, 256, w - 88, h - 272, o.charola, 8)
      E.perillas(d, c, m, { x = 50, y = 264, w = w - 100, claves = o.claves, por = por, color = o.color, metal = o.metal, alto = 92, r = 23 })
    end,
  }
end

-- Equipo de bulbos: brillan con el sonido y con lo que se les empuja.
function E.bulbos(o)
  local por = o.por or math.min(7, #o.claves)
  local w = 640
  local h = 190 + filas(#o.claves, por) * 92 + 20
  return {
    ancho = w, alto = h,
    dibujar = function(d, c, m)
      frente(d, m, o, w, h)
      d.hueco(48, 46, 196, 128, 8)
      local empuje = o.empuje and o.empuje(c) or 0.5
      local brilla = m.activo and (0.18 + 0.3 * empuje + math.min(0.6, m.nivel * (0.5 + empuje))) or 0
      for i = 0, 2 do d.valvula(60 + i * 60, 54, 50, 114, brilla) end
      d.grafica(258, 46, w - 258 - 92, 128, o.que)
      d.bandeja(44, 190, w - 88, h - 206, o.charola, 8)
      E.perillas(d, c, m, { x = 50, y = 198, w = w - 100, claves = o.claves, por = por, color = o.color, metal = o.metal, alto = 92, r = 23 })
    end,
  }
end

return E
