Beethoven Craft 0.12.4: plugins de verdad, editor de audio y todo MIT o Apache.

Beethoven DAW:
• Editor de audio (F2): corta, normaliza, limpia ruido, cambia tono o duración sin cambiar lo otro. Tu archivo no se toca.
• 48 efectos con ventana de equipo analógico: ecualizador paramétrico, reverb de convolución, vocoder, compresor lateral, cinta, bulbos, pedales… Los nombres ahora dicen lo que hacen; tus proyectos los encuentran igual.
• Sintetizadores propios (FM de 6 operadores, 3 osciladores, mono analógico…), muestreador, batería de 16 pads y cortador de ritmos.
• Afinador de voz, quita ruido, arpegiador y acordes en vivo.
• Controladores: mueve cualquier perilla con el nivel, un LFO o una fórmula.
• El Mixer empareja las pistas cuyos efectos se tardan.
• Corregido: algunos efectos se quedaban sonando al detener o al acabar el clip.
• Corregido: los instrumentos VST3 y CLAP salían en silencio dentro de un proyecto.

Haz y comparte plugins:
• Suelta una carpeta, un .zip o un .lua y funciona, sin reiniciar; los cambios se oyen al guardar.
• Lua puede correr mientras suena, muestra por muestra o por frecuencias. Guía: daw-demo/PLUGINS.md
• Nuestros efectos e instrumentos también como VST3 y CLAP, «Beethoven …», para usarlos en otros programas (descarga BeethovenPlugins). Es la primera versión: cuéntanos si alguno falla.

Movie:
• Los efectos de sonido, el sintetizador, los controles y los íconos son nuevos. Los proyectos abren igual, pero algunos efectos e instrumentos suenan un poco distinto.

Licencia: todo el código propio es MIT o Apache-2.0. Lo que venía de Lemon Studio (GPL-3.0) se quitó; las versiones hasta la 0.12.0 siguen bajo GPL.

Cómo instalar:
• Mac (Apple Silicon): descomprime y arrastra «Beethoven Craft.app» a Aplicaciones. La app no está firmada por Apple: la primera vez ábrela con clic derecho → Abrir (o en Terminal: xattr -dr com.apple.quarantine "/Applications/Beethoven Craft.app"). Necesita ffmpeg: brew install ffmpeg
• Windows: corre el instalador (trae ffmpeg).
• Linux: descomprime y abre «beethoven-craft». Necesita ffmpeg: sudo apt install ffmpeg
