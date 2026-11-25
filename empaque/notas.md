Beethoven Craft 0.12: un DAW nuevo, un asistente de IA y vistas previas al instante.

Importante: Beethoven DAW se cambió por uno nuevo, escrito desde cero. Las canciones del DAW anterior (.beethoven) no se abren en este: tus archivos no se borran, y para seguir con ellas puedes usar la 0.11 (sigue en la página de versiones). El nuevo guarda en .bdaw y abre .flp y MIDI. Todavía no trae Partitura, Ideas ni VST2.

Beethoven DAW:
• Playlist siempre al fondo; Mixer como pestaña o ventana; Ritmo, Piano roll e Instrumento encima.
• Unos 195 instrumentos grabados nota por nota, por familia (banco MuseScore General).
• Pistas de audio (WAV, MP3, OGG, FLAC, M4A, video…), grabación, Mixer con envíos y efectos.
• Crea tus propios instrumentos y efectos en Lua, con su propia interfaz, imágenes y logo.
• Plugins VST3 y CLAP, SoundFont, SFZ y teclados MIDI.
• Exporta WAV, pistas separadas, MP3, FLAC, OGG y M4A.

Asistente de IA, en Movie y en el DAW:
• Pídele cortes, títulos, efectos, ritmos o mezclas con tus palabras; todo se puede deshacer.
• Usa la IA que tú tengas: Claude Code, una clave de API u Ollama y LM Studio en tu computadora. Si no la usas, no se manda nada.
• Solo puede tocar el proyecto abierto. Ver cuadros del video o buscar tus archivos se permite aparte, en sus Ajustes.

Movie:
• Al meter un video, las miniaturas de efectos, filtros y transiciones se hacen solas, sin abrir el catálogo, también mientras reproduces.
• Las miniaturas del clip aparecen conforme se leen.
• Nodos con clips e imágenes, clases de clips y línea de tiempo con vidrio.

Cómo instalar:
• Mac (Apple Silicon): descomprime y arrastra «Beethoven Craft.app» a Aplicaciones. La app no está firmada por Apple: la primera vez ábrela con clic derecho → Abrir (o en Terminal: xattr -dr com.apple.quarantine "/Applications/Beethoven Craft.app"). Necesita ffmpeg: brew install ffmpeg
• Windows: corre el instalador (trae ffmpeg).
• Linux: descomprime y abre «beethoven-craft». Necesita ffmpeg: sudo apt install ffmpeg
