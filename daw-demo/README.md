# Beethoven DAW Demo

Estudio de música propio de Beethoven Craft, en demostración; es el DAW que abre el menú de Beethoven Craft. Español e inglés, según el sistema o lo que se elija en **Archivo › Idioma**. *(English below.)*

## Qué hace

- **La Playlist (F5) es el fondo** y ocupa toda la pantalla; el **Mixer** (F9) es otra pestaña, que se puede sacar como ventana. **Ritmo** (F6), **Piano roll** (F7) e **Instrumento** (F8) son ventanas encima, en el menú **Ventana**, que también se pueden fijar como pestaña. Todo con el metal y el vidrio de Movie. Abajo siempre dice qué hace el ratón en ese lugar.
- **Pistas de audio**: arrastra o pega (⌘V) WAV, MP3, OGG, Opus, FLAC, M4A, AIFF, CAF o un video. Cada audio toma su propia pista del Mixer. Soltado en la Playlist cae donde está el ratón; en una tira del Mixer, va a esa pista; en Ritmo o Instrumento, se vuelve instrumento. Se mueven, recortan, cortan y ajustan (volumen, lado, fundidos, velocidad). **Graba** el micrófono.
- **Editor de audio** (F2, o doble clic en un audio): selecciona sobre la onda, corta, copia, pega, recorta y silencia; normaliza y hace fundidos; quita un ruido que le enseñas, acorta silencios, voltea al revés, y cambia la duración sin cambiar el tono o el tono sin cambiar la duración. Tu archivo original no se toca.
- **Mixer** de 12 pistas, 4 envíos y general: ranuras de efectos que abren su ventana con un clic, cuánto efecto por ranura, ecualizador de tres bandas por pista, ancho, polaridad, mudo, solo, medidores con aviso de saturación. 48 efectos: ecualizador paramétrico de seis bandas (cada una del tipo que quieras) con cortes y analizador, reverb de convolución con ocho lugares o tu propio audio, vocoder, compresor lateral, compresor con su curva, limitador, compuerta, multibanda, quita eses, transitorios, reverb, ecos, coro, flanger, faser, trémolo, cinta, lo-fi y más. Cada uno trae preajustes y una gráfica medida con el propio efecto. Cada uno con su ventana de aparato (rack de estudio, pedal, grabadora de cinta, bulbos) y medidores que leen lo que de verdad hace.
- **Controladores**: por nivel, LFO y con fórmula. Clic derecho en una perilla de un plugin › «Moverlo con…».
- **Pasar la pista a audio**: clic derecho en el nombre de una tira del Mixer.
- **Línea de tiempo suave**: los clips siguen al ratón y caen en su lugar; imán, tijera (⌘3), pincel (⌘2), borrador (⌘4), elegir varios, copiar, pegar, duplicar (⌘D), partir en el cabezal (⌘E), recortar de los dos lados, fundidos en las esquinas, mapa abajo.
- **Elegir instrumento** al crear un patrón o una pista, y cambiarlo desde el Piano roll.
- **Asistente de IA** (F10): le dices qué quieres y edita el proyecto. Encuentra solo lo que tengas (Claude Code, Ollama, LM Studio, llama.cpp, o claves en el entorno); también acepta tu clave o cualquier servidor con su dirección y puerto. Mide el audio y la mezcla (niveles, silencios, tempo, tonalidad), maneja plugins, exporta y sabe qué tienes seleccionado. Todo lo que hace se puede deshacer. El mismo asistente está en Movie.
- **Instrumentos grabados**: unos 195, nota por nota, del banco **MuseScore General** (MIT): pianos, guitarras, bajos, cuerdas y secciones de orquesta, metales, maderas, coros, sintetizadores, instrumentos del mundo, baterías y percusión. Van por familia en la izquierda. El banco (40 MB) va dentro de la app; al compilar lo baja `empaque/sonidos.sh` a `sonidos/`.
- **Sintetizadores propios**, con ventana de metal y preajustes: 3 osciladores, FM de 6 operadores, supersierra, bajo ácido, bombo y 808, cuerda pulsada y órgano de barras. Y con tus audios: **muestreador**, **batería de 16 pads** y **cortador de ritmos** (encuentra los golpes y los escribe en el patrón).
- **Tus plugins**: suelta una carpeta, un `.zip` o un `.lua` y funciona, sin reiniciar; los cambios se oyen al guardar. Guía para hacerlos y compartirlos: [PLUGINS.md](PLUGINS.md). Con editor dentro de la app; también herramientas de notas. Los efectos de fábrica son de Lua.
- **Plugins VST3 y CLAP**, con su ventana (Mac) o sus controles.
- **SoundFont (.sf2 y .sf3), SFZ y audio suelto** como instrumentos. **Teclados MIDI** y anotar lo que tocas.
- **Ritmos listos**, reparto parejo, metrónomo, swing, escalas.
- **Abre `.flp`**: tempo, canales, notas, patrones y clips de patrón. **MIDI** de entrada y salida.
- **Exporta** WAV y pistas separadas; MP3, FLAC, OGG y M4A si ffmpeg está instalado.

Lo que todavía no: automatización, la ventana propia de los plugins en Windows y Linux, VST2 y Audio Units, SFZ con grabaciones comprimidas raras. Los plugins internos de FL Studio son cerrados y solo abren dentro de FL.

## Compilar y probar

    cargo run --release
    cargo test --release
    cargo run --release --example plugins     # prueba los plugins instalados
    cargo run --release -- --medir            # velocidad del motor
    empaque/empacar.sh                        # arma la app de Mac

## Hacer un instrumento

Hay grabaciones y sintetizadores de fábrica; en **Tus plugins** (a la izquierda) escribes los tuyos. Clic derecho en uno de fábrica para ver cómo está hecho.

Un guion arma el sonido **una vez**; después lo toca el motor, sin Lua en el hilo de audio. Se guardan en `Music/Beethoven DAW Demo/Lua`, cada uno en su carpeta (ver abajo).

```lua
return {
  nombre = { es = "Mi bajo", en = "My bass" },   -- o solo "Mi bajo"
  tipo = "instrumento",                           -- instrumento, efecto o herramienta
  grupo = "Míos",
  voces = 8,
  controles = { { "corte", "Corte", 100, 8000, 900 } },  -- clave, nombre, mínimo, máximo, valor
  sonido = function(n, c)                         -- n.hz, n.fuerza, n.azar
    local x = sierra(n.hz) + pulso(n.hz / 2, 0.5)
    return pasabajos(x, c.corte, 0.3) * adsr(0.01, 0.2, 0.6, 0.1) * n.fuerza * 0.4
  end,                                            -- una señal, o dos (izquierda, derecha)
}
```

| Función | Qué da |
| --- | --- |
| `seno(hz, fase)` `sierra(hz, fase)` `triangulo(hz, fase)` `pulso(hz, ancho)` `ruido()` | osciladores (la fase va en vueltas: sirve para FM) |
| `adsr(ataque, caída, sostén, soltar)` `golpe(segundos)` | envolventes |
| `pasabajos(x, hz, filo)` `pasaaltos` `pasabanda` | filtros |
| `graves(x, hz, dB, ancho)` `campana` `agudos` `muesca(x, hz, ancho)` | bandas de ecualizador |
| `seguidor(x, ataque, soltar)` `db(x)` | nivel y decibeles (compresores, compuertas) |
| `escalon(x, n)` `retener(x, hz)` `pasatodo(x, hz)` | triturar, muestrear, mover la fase |
| `retardo(x, seg, repite, tope)` `sala(x, tamaño, brillo)` `cuerda(x, hz, caída, brillo)` | espacio y cuerda |
| `muestra(velocidad, "izq"/"der")` | la grabación del canal (WAV, SF2, SFZ) |
| `tanh` `abs` `sen` `minimo` `maximo` `mezclar(a, b, k)` y `+ - * / ^` | cuentas |

### Inventa lo tuyo

| Función | Qué da |
| --- | --- |
| `curva(x, function(v) … end, "suave")` | tu propia distorsión: Lua llena la curva (de -4 a 4) al armarse, con los `if` y cuentas que quieras. `"suave"` hace que raspe menos |
| `onda(hz, function(fase) … end, desfase)` | tu propio oscilador: una vuelta, `fase` de 0 a 1; los agudos no rebotan |
| `lazo(seg, function(vuelta) return … end, tope)` | un lazo de realimentación con lo que quieras dentro (filtros, curvas, otro `lazo`): ecos, peines, reverbs. Mínimo 64 muestras |
| `convolucion(x, "izq"/"der")` | el sonido dentro de un lugar: el del control `espacio` (0 a 7) o el audio que se le cargue; `largo` y `reves` también cuentan |
| `mide("nombre", x)` | pasa igual y deja ver su valor en la interfaz (`m.medidas.nombre`). Si se llama `"control"`, sirve para mover perillas de otros |
| `e.otra_izq` `e.otra_der` | la pista que diga el control `pista`: compresores laterales, vocoders |
| `tempo()` `negras(n)` `posicion()` | el tempo, cuántos segundos duran `n` negras y en qué negra va la canción |
| `raiz` `ln` `exp` `piso` `resto(a, b)` `mayor(a, b)` `si(cual, a, b)` | más cuentas y decisiones |

```lua
-- Eco a tempo, oscuro en cada vuelta
return {
  nombre = "Eco a tempo", tipo = "efecto", grupo = "Míos",
  controles = { { "repite", "Repite", 0, 0.9, 0.5 }, { "oscuro", "Oscuro Hz", 500, 12000, 4000 } },
  sonido = function(e, c)
    local function f(x)
      return x + lazo(negras(0.75), function(vuelta)
        return pasabajos(x + vuelta * c.repite, c.oscuro, 0)
      end, 2)
    end
    return f(e.izq), f(e.der)
  end,
}
```

En su interfaz, además: `m.onda` (256 puntos de lo que suena) y `m.espectro` (64, de graves a agudos) para dibujar tus medidores; `m.apretado`, `m.clic`, `m.x`, `m.y` y `d.poner(clave, valor)` para hacer tus propios botones, listas o arrastres. `presets = { { nombre = "…", clave = valor } }` le pone preajustes.

En un **efecto**, `sonido(e, c)` recibe `e.izq` y `e.der`. Una **herramienta** tiene `notas(lista, c)`, con `paso`, `largo`, `tono` y `fuerza` de cada nota, y devuelve la lista nueva.

### Con su propia interfaz

Un instrumento o efecto puede dibujar su ventana y elegir su estilo. `dibujar` corre en cada cuadro; mira `efectos/vozencaja/vozencaja.lua` («Voz en caja»).

```lua
interfaz = {
  ancho = 300, alto = 160,
  dibujar = function(d, c, m)   -- c: valores de los controles · m: tiempo, nivel, tocando, idioma, x, y
    d.degradado(0, 0, 300, 160, "#2a2018", "#0c0a08", 8)
    d.texto(150, 20, "MI EFECTO", 20, "#ffd95e", "centro")
    d.perilla(80, 90, 30, "corte", "#ff6a3c")
    d.circulo(220, 90, 20 + m.nivel * 30, "#ffd95e")
  end,
}
```

| Pinta | Se mueve |
| --- | --- |
| `caja(x,y,w,h,color,r)` `degradado(x,y,w,h,arriba,abajo,r)` `borde(x,y,w,h,color,grueso,r)` `circulo(x,y,r,color)` `aro(x,y,r,color,grueso)` `linea({x,y,…},color,grueso)` `poligono({x,y,…},color)` `texto(x,y,s,tamaño,color,"centro"/"derecha")` `metal(x,y,w,h,r)` `hueco(…)` `vidrio(x,y,w,h,color,r)` `tornillo(x,y,r)` | `perilla(x,y,r,clave,color,metal)` (`metal` es el color de la tapa: "#b4b7bd" cromo, "#a8823c" latón) `tirador(x,y,w,h,clave,color)` `boton(x,y,w,h,texto,clave,color)` `pad(x,y,w,h,claveX,claveY)` |

`incluir("equipo")` arma ventanas completas: `equipo.rack{…}`, `equipo.pedal{…}`, `equipo.cinta{…}` y `equipo.bulbos{…}` (mira `efectos/equipo.lua`). Otras piezas: `grafica`, `prender`, `valvula`, `carrete`, `asa`, `pista`, `archivo`, `pads`, `teclado` y `escribir`.

Piezas de metal: `acero(x,y,w,h,tinte,r)` (panel cepillado con tornillos), `bandeja(x,y,w,h,tinte,r)` (charola fresada), `hueco(…)` (pantalla de vidrio), `grabado(x,y,s,tamaño,color,"centro")` (letras fresadas), `pisoton(x,y,r,clave,color)` (interruptor de pie), `led(x,y,r,color,brillo)`, `aguja(x,y,w,h,valor,"NIVEL")` (medidor), `palanca(x,y,clave,color)` y `lista(x,y,w,h,clave,{"A","B"},color)` (teclas para elegir una).

`imagen(x,y,w,h,"fondo.png",opacidad)` pinta un PNG de la carpeta del guion.

### Su carpeta: imágenes, logo y otros Lua

Cada guion vive en una carpeta con su nombre, por ejemplo `Lua/voz gruesa/`:

```
voz gruesa/
  voz gruesa.lua    -- el principal (o el único .lua)
  ayuda.lua         -- se usa con incluir("ayuda")
  logo.png          -- logo = "logo.png" en el guion
  fondo.png
  fondos/noche.png  -- también una subcarpeta
```

- `logo = "logo.png"` lo muestra en la lista, en el menú de efectos y en su ventana.
- `local ayuda = incluir("ayuda")` corre otro `.lua` de la misma carpeta y devuelve lo que regrese.
- Solo PNG, hasta 2048 px por lado, 8 MB cada uno y 32 MB por carpeta. El guion solo ve los nombres de su carpeta.
- En el editor, **Abrir su carpeta** la muestra para que pongas ahí tus imágenes. Si editas uno de fábrica, tu copia se lleva sus imágenes.
- Mira `efectos/vozgruesa/` («Voz gruesa»). Los `.lua` sueltos de antes siguen sirviendo.

Los guiones no pueden abrir archivos ni usar el sistema, y tienen tope de memoria y de pasos. Los plugins VST3 y CLAP sí son programas completos: instala solo los de confianza.

## El asistente y tus datos

- A la IA se le mandan los nombres, notas y ajustes del proyecto; nunca el audio.
- Con **Claude Code** se usa el que tienes instalado, con tu cuenta, y sin sus herramientas de archivos ni de terminal: solo puede usar las del proyecto.
- Las claves se guardan en `Music/Beethoven DAW Demo/asistente.json`, que solo tú puedes leer, y solo se mandan a la dirección que pusiste (con `https`, o en tu propia máquina).
- El servidor **MCP** (para que otro programa controle este) solo escucha en tu máquina y pide una llave.

## English

Beethoven Craft's own music studio, as a demo. Arrangement, Rhythm, Notes and Mixer; instruments, effects and note tools written in **Lua**; **VST3 and CLAP** plugins; **SoundFont, SFZ and WAV** instruments; opens **.flp** (tempo, channels, notes, patterns and pattern clips) and **MIDI**; exports WAV and MIDI. An **AI assistant** (F10) edits the project for you with Claude Code, Ollama, LM Studio, an API key or any compatible server. Lua scripts can draw their own interface; each one lives in its own folder with its PNG images, a logo and helper Lua files (`incluir`). It cannot load FL Studio's internal plugins, VST2 or Audio Units. The language follows the system, or **File › Language**. Function names in Lua are in Spanish; see the table above.

## Licencia · License

MIT o / or Apache-2.0. Escrito desde cero / written from scratch: [AVISOS.md](AVISOS.md).
