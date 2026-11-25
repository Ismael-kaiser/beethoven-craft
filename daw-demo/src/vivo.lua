-- Lo que tiene a la mano un guion en vivo, además de math, string y table. No pide memoria al sonar.
local unpack = table.unpack

-- Una cola para retardos: pone una muestra y lee cualquiera de las anteriores, también entre dos.
function retraso(n)
  local t, p = {}, 1
  for i = 1, n do t[i] = 0.0 end
  return {
    pon = function(x) t[p] = x p = p % n + 1 end,
    lee = function(d)
      if d < 1 then d = 1 elseif d > n - 2 then d = n - 2 end
      local e = d // 1
      local f = d - e
      local i = (p - 1 - e) % n + 1
      local j = (i - 2) % n + 1
      i, j = i // 1 | 0, j // 1 | 0
      return t[i] + (t[j] - t[i]) * f
    end,
  }
end

function db(x) return 10 ^ (x / 20) end
function tanh(x)
  if x > 4 then return 1.0 elseif x < -4 then return -1.0 end
  local q = x * x
  return x * (135135 + q * (17325 + q * (378 + q))) / (135135 + q * (62370 + q * (3150 + q * 28)))
end
function hz(tono) return 440 * 2 ^ ((tono - 69) / 12) end

-- Procesa por frecuencias: f(re, im, lado, c, m) cambia el espectro en su lugar. `n` es potencia de dos, hasta 4096.
function espectral(n, f)
  local salto = n // 4
  local w, entra, sale, re, im = {}, { {}, {} }, { {}, {} }, {}, {}
  for i = 1, n do
    w[i] = math.sin(math.pi * (i - 1) / n)
    re[i], im[i] = 0.0, 0.0
    for l = 1, 2 do entra[l][i], sale[l][i] = 0.0, 0.0 end
  end
  local k = 0
  return {
    latencia = n,
    bloque = function(izq, der, cuantas, c, m)
      for q = 1, cuantas do
        -- `entra` es circular: k dice por dónde va.
        local i = k % n + 1
        local xi, xd = izq[q], der[q]
        izq[q], der[q] = sale[1][i], sale[2][i]
        sale[1][i], sale[2][i] = 0.0, 0.0
        entra[1][i], entra[2][i] = xi, xd
        k = k + 1
        -- Un lado a la vez, a media vuelta uno del otro: así no se junta todo el trabajo.
        for l = 1, 2 do
          if (k + (l - 1) * (salto // 2)) % salto == 0 then
            local e, s = entra[l], sale[l]
            for j = 1, n do
              re[j] = e[(k + j - 1) % n + 1] * w[j]
              im[j] = 0.0
            end
            fft(re, im)
            f(re, im, l, c, m)
            fft(re, im, true)
            for j = 1, n do
              local o = (k + j - 1) % n + 1
              s[o] = s[o] + re[j] * w[j] * 0.5
            end
          end
        end
      end
    end,
  }
end

function __por_muestra(f)
  return function(izq, der, n, c, m)
    for i = 1, n do
      local a, b = f(izq[i], der[i], c, m, i)
      izq[i], der[i] = a, b or a
    end
  end
end
