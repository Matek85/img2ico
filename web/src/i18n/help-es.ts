// Spanish help texts: the same keys and marks as help-en.ts, with the names of the buttons as they read in es.ts.
// Translated by an AI.
import type { helpEn } from './help-en.ts';

export const helpEs: Record<keyof typeof helpEn, string> = {
  // ---- Primeros pasos
  'help.start.01.h2': 'Abrir una imagen',
  'help.start.02.p':
    'Suelta una imagen en la página, pulsa **Elegir un archivo** o pega una desde el portapapeles (una captura de pantalla, por ejemplo) con [[Ctrl]] + [[V]] ([[Cmd]] + [[V]] en un Mac). img2ico lee imágenes PNG, JPG, GIF, WebP, BMP, TIFF, SVG e ICNS; [Tipos de archivo](help:file-types) explica para qué sirve cada una.',
  'help.start.03.p':
    'Varias imágenes, o un ZIP lleno de ellas, pasan a una cola de una vez (más abajo). Un archivo .ico que sueltes se abre, en cambio, para mirar dentro: puedes revisarlo, sacar imágenes de él o editar su imagen más grande como imagen.',
  'help.start.04.h2': 'Elegir para qué es el icono',
  'help.start.05.p':
    'Los ajustes junto a la vista previa empiezan con un **Inicio rápido**. En **¿Para qué es?** elige un conjunto de tamaños: Aplicación de Windows, Estándar, Icono para sitio web, Aplicación de macOS o Prueba rápida. En **Estilo** elige cómo se coloca la imagen en su cuadrado: Sencillo, Icono de app redondeado o Redondo.',
  'help.start.06.p':
    'La vista previa cambia al instante. Los ajustes se pliegan en una barra estrecha de iconos para dejar sitio a la vista previa; la flecha de su parte superior los abre, y el **Editor avanzado** tiene todos los ajustes ([los ajustes explicados](help:settings)).',
  'help.start.07.h2': 'Revisar el resultado',
  'help.start.08.p':
    'La vista previa muestra cada tamaño del icono a su tamaño real. La columna de cuadrados a su izquierda cambia el fondo detrás del icono: cuadros, claro, oscuro, gris o un color propio (señala uno para probarlo, haz clic para conservarlo). **Icono** muestra los iconos, **Antes y después** pone el original junto al resultado bajo un divisor que puedes arrastrar, y **Píxeles** muestra el icono píxel a píxel con el color exacto de cada uno.',
  'help.start.09.p':
    'Los avisos bajo la vista previa te dicen cuándo algo probablemente se verá mal, por ejemplo cuando la imagen es más pequeña que un tamaño que elegiste.',
  'help.start.10.h2': 'Cambiar la propia imagen',
  'help.start.11.p':
    '**Editar**, sobre la vista previa, abre las herramientas de imagen: usar solo una parte de la imagen, girarla y ver cómo la recorta un estilo redondo. Se describen en [los ajustes explicados](help:settings).',
  'help.start.12.h2': 'Descargar',
  'help.start.13.p':
    'Sobre la vista previa, elige el tipo de archivo: **Windows .ico**, **macOS .icns** o **ZIP para sitio web**, y pulsa el botón de descarga. La barra se mantiene a la vista mientras te desplazas. El botón de archivo junto a él guarda cada tamaño como un archivo PNG, en un ZIP.',
  'help.start.14.h2': 'Crear varios iconos',
  'help.start.15.p':
    '**Añadir a la cola** guarda el icono y te deja elegir la siguiente imagen. La cola está junto al editor: cambia el orden (arrastra una fila por su asa o usa los botones de flecha), abre cualquier icono para cambiarlo otra vez (los cambios se guardan en él solos), compara dos iconos y descárgalos todos como un ZIP o uno a uno. Mientras editas un icono de la cola, **Usar para todos los iconos** vuelve a crear los demás iconos con los tamaños, el aspecto o la eliminación del fondo de este. La cola solo existe mientras la página está abierta.',
  'help.start.16.h2': 'Tus ajustes se recuerdan',
  'help.start.17.p':
    'La página recuerda tus últimos ajustes en este navegador, para que la siguiente imagen empiece como lo dejaste. **Restablecer valores**, arriba a la derecha del editor, lo devuelve todo (pregunta antes). El recorte y el giro pertenecen a una imagen y no se recuerdan. [Privacidad](help:privacy) explica qué guarda el navegador.',
  'help.start.18.h2': 'Idioma y modo claro u oscuro',
  'help.start.19.p':
    'El menú **Idioma** de la barra superior muestra la página en inglés, alemán, español, portugués de Brasil o francés. La primera vez, la página elige el idioma de tu navegador; tu elección se recuerda. Algunas traducciones las ha hecho una IA, y la página lo indica arriba. El círculo medio lleno de al lado cambia entre el modo claro y el oscuro; hasta que lo uses, la página sigue a tu sistema.',

  // ---- Los ajustes explicados
  'help.settings.01.h2': 'Inicio rápido',
  'help.settings.02.h3': '¿Para qué es?',
  'help.settings.03.dl.1.t': 'Aplicación de Windows',
  'help.settings.03.dl.1.d':
    'Los diez tamaños que recomienda Microsoft (16, 20, 24, 32, 40, 48, 64, 96, 128 y 256 píxeles), para que Windows nunca tenga que estirar un icono. Un archivo .ico.',
  'help.settings.03.dl.2.t': 'Estándar',
  'help.settings.03.dl.2.d': 'Seis tamaños de 16 a 256 píxeles: lo que incluyen la mayoría de los programas. Un archivo .ico.',
  'help.settings.03.dl.3.t': 'Icono para sitio web',
  'help.settings.03.dl.3.d':
    'Todo lo que necesita un sitio web, en un ZIP: favicon.ico, iconos PNG, un icono de Apple y un manifiesto web ([Tipos de archivo](help:file-types) los enumera).',
  'help.settings.03.dl.4.t': 'Aplicación de macOS',
  'help.settings.03.dl.4.d':
    'Un archivo .icns para macOS. Tiene su propio conjunto de tamaños (de 16 a 1024 píxeles), así que los tamaños que elijas solo deciden lo que muestra la vista previa.',
  'help.settings.03.dl.5.t': 'Prueba rápida',
  'help.settings.03.dl.5.d': 'Solo 16 y 32 píxeles: un archivo pequeño, para probar cosas.',
  'help.settings.04.h3': 'Estilo',
  'help.settings.05.dl.1.t': 'Sencillo',
  'help.settings.05.dl.1.d': 'La imagen tal cual, sin margen ni redondeo.',
  'help.settings.05.dl.2.t': 'Icono de app redondeado',
  'help.settings.05.dl.2.d': 'Un pequeño margen y esquinas redondeadas, como un icono de aplicación.',
  'help.settings.05.dl.3.t': 'Redondo',
  'help.settings.05.dl.3.d': 'Llena un círculo: se conserva el centro de la imagen y el resto se recorta.',

  'help.settings.06.h2': 'Tamaños',
  'help.settings.07.p':
    'Un archivo .ico contiene varias imágenes, una por tamaño, en píxeles por lado. Windows usa el tamaño más cercano que encuentra para la barra de tareas, las barras de título, las listas y las vistas previas grandes, así que más tamaños significan menos imágenes estiradas. En el Editor avanzado puedes marcar cualquiera de 16, 20, 24, 32, 40, 48, 64, 96, 128 y 256 píxeles. Cuando la imagen es más pequeña que un tamaño que elegiste, un aviso te lo dice: ese tamaño se amplía y se ve borroso.',

  'help.settings.08.h2': 'Aspecto',
  'help.settings.09.dl.1.t': 'Margen',
  'help.settings.09.dl.1.d':
    'Espacio vacío alrededor de la imagen, como proporción del icono (0 a 40 %). Un icono que llena su cuadrado hasta el borde se ve apretado junto a otros.',
  'help.settings.09.dl.2.t': 'Esquinas redondeadas',
  'help.settings.09.dl.2.d': 'Cuán redondas son las esquinas, como proporción del lado más corto (0 a 50 %). El 50 % da un círculo.',
  'help.settings.09.dl.3.t': 'Ajuste',
  'help.settings.09.dl.3.d':
    '**Mostrar toda la imagen** la conserva entera y deja franjas transparentes donde la imagen no es cuadrada. **Llenar el cuadrado** recorta lo que sobresale, para que el cuadrado quede lleno.',
  'help.settings.09.dl.4.t': 'Blanco y negro',
  'help.settings.09.dl.4.d': 'Quita todo el color y conserva el brillo.',
  'help.settings.09.dl.5.t': 'Recortar el margen vacío',
  'help.settings.09.dl.5.d':
    'Corta el borde transparente alrededor del motivo para que llene el icono (después del recorte, si lo hay). Útil tras quitar un fondo.',

  'help.settings.10.h2': 'Quitar el fondo',
  'help.settings.11.p':
    'Marca **Quitar el fondo** para hacer transparente un fondo liso. De forma predeterminada el color se detecta en el borde de la imagen; también puedes elegir el color tú mismo o tomarlo de la imagen: haz clic en un píxel en la vista **Píxeles**, y los ajustes se abren y parpadean donde fue a parar el color.',
  'help.settings.12.dl.1.t': 'Tolerancia',
  'help.settings.12.dl.1.d':
    'Cuánto puede diferir un color del color de fondo y seguir contando como fondo (0 a 100). Un valor más alto quita más, pero puede comerse el motivo.',
  'help.settings.12.dl.2.t': 'Borde suave',
  'help.settings.12.dl.2.d': 'Cuán suave es el borde de lo que se quita (0 a 100). 0 es un borde duro.',
  'help.settings.13.p':
    'La eliminación empieza en el borde de la imagen y avanza hacia dentro, así que un color que aparece dentro del motivo se queda, a menos que toque la zona quitada.',

  'help.settings.14.h2': 'Editar la imagen',
  'help.settings.15.p':
    '**Editar** (tecla [[C]]) muestra la imagen con un marco, y solo se usa lo que queda dentro del marco. Pulsa **Listo** o [[Esc]] cuando hayas terminado; **Usar toda la imagen** vuelve a poner el marco alrededor de todo.',
  'help.settings.16.dl.1.t': 'El candado',
  'help.settings.16.dl.1.d':
    'Con el candado cerrado (al principio) el marco se queda en el centro de la vista previa y mueves la imagen bajo él: arrástrala, haz zoom con la rueda del ratón o usa las flechas con [[+]] y [[-]]. Abre el candado ([[Espacio]]) para arrastrar en su lugar el marco y sus tiradores.',
  'help.settings.16.dl.2.t': 'Forma',
  'help.settings.16.dl.2.d': 'Libre, 1:1, 4:3, 3:2 o 16:9. Una forma fija mantiene su proporción mientras trabajas.',
  'help.settings.16.dl.3.t': 'Colocar el marco',
  'help.settings.16.dl.3.d':
    'El panel de 3 × 3 pone el marco en un borde, una esquina o el centro de la imagen y muestra dónde está. El teclado numérico hace lo mismo.',
  'help.settings.16.dl.4.t': 'Marco más grande, girar el marco',
  'help.settings.16.dl.4.d':
    '**Marco más grande** hace el marco tan grande como permite la imagen, con la misma forma. **Girar el marco** intercambia ancho y alto.',
  'help.settings.16.dl.5.t': 'Girar y reflejar la imagen',
  'help.settings.16.dl.5.d':
    'Mientras el candado está cerrado, un control deslizante gira la imagen de −180° a 180° en pasos de 1° (un ángulo positivo gira en sentido horario), y unos botones la giran 90° o 180°. La imagen gira bajo el marco y las esquinas que deja el giro son transparentes. Dos botones reflejan la imagen, de izquierda a derecha y de arriba abajo; primero se refleja y después se gira.',
  'help.settings.16.dl.6.t': 'Izquierda, Arriba, Ancho, Alto',
  'help.settings.16.dl.6.d': 'El marco en píxeles de la imagen (después del giro). Escribe números exactos si los necesitas.',
  'help.settings.16.dl.7.t': 'Mostrar la forma',
  'help.settings.16.dl.7.d':
    'Con el estilo Icono de app redondeado o Redondo, las esquinas que el estilo recorta se sombrean dentro del marco, para que veas cómo quedará el icono. Puedes desactivarlo.',
  'help.settings.17.p':
    'El recorte y el giro pertenecen a la imagen que tienes delante; no se recuerdan para la siguiente. Todas las teclas de estas herramientas están en [atajos de teclado](help:keyboard-shortcuts).',

  'help.settings.18.h2': 'GIF animados',
  'help.settings.19.p':
    'Un GIF animado muestra un pequeño reproductor sobre la vista previa: reproducir, pausar, recorrer los fotogramas y detenerte en el que quieras. El icono se crea a partir del fotograma en el que te detengas.',

  'help.settings.20.h2': 'El paquete para sitios web',
  'help.settings.21.p':
    'Con **ZIP para sitio web**, el engranaje abre los ajustes del paquete: el nombre del sitio (se muestra cuando el sitio se añade a una pantalla de inicio), el color del tema y el fondo del icono de Apple (un iPhone rellena las zonas transparentes de negro, así que el icono de Apple se coloca sobre este color).',

  'help.settings.22.h2': 'La columna de ajustes',
  'help.settings.23.p':
    'La columna se pliega en una barra de iconos para que la vista previa pueda ser ancha; la flecha de su parte superior la abre. Al iniciar las herramientas de imagen se pliega sola y se vuelve a abrir cuando terminas. **Restablecer valores** pregunta antes y luego devuelve tamaños, aspecto, fondo, recorte y giro al principio.',
  'help.settings.24.h3': 'Los ajustes como archivo',
  'help.settings.25.p':
    '**Exportar ajustes**, junto a Restablecer valores, guarda los tamaños, el margen, las esquinas, el ajuste, el blanco y negro, la eliminación del fondo y el tipo de archivo en un pequeño archivo de texto llamado `img2ico.toml`. Es el mismo tipo de archivo que lee la línea de comandos, así que la exportación es compatible con ella: `img2ico --config img2ico.toml logo.png`, o deja el archivo en tu carpeta y la línea de comandos lo encuentra sola. A la inversa, **Importar ajustes** toma un archivo guardado aquí o escrito para la línea de comandos, muestra los ajustes e indica lo que no tomó (las opciones de la línea de comandos que aquí no sirven). Una línea que la página no conoce, o un valor que no está permitido, detiene la importación con una lista, y no se cambia nada. El reflejo, el giro, el recorte y el fotograma de un GIF están en el archivo cuando los has usado, así que un archivo guardado aquí da el mismo icono al importarlo de nuevo, y también en la línea de comandos (que conoce `--flip-horizontal`, `--flip-vertical`, `--rotate`, `--crop` y `--gif-frame`). Un recorte que no cabe en la imagen a la que importas se omite, y un aviso lo dice.',
  'help.settings.26.h3': 'Deshacer y rehacer',
  'help.settings.27.p':
    'Las dos flechas arriba a la derecha del editor, **Ctrl+Z** (Cmd+Z en un Mac) y **Ctrl+Mayús+Z** o **Ctrl+Y** deshacen y rehacen los últimos cambios de los ajustes, incluidos el marco de recorte, el giro y el reflejo. Una serie de cambios pequeños, como arrastrar un control deslizante, cuenta como uno. **Restablecer valores** e **Importar ajustes** también se pueden deshacer. Debajo del editor, una lista de cambios muestra cada paso. Un clic vuelve (o avanza) varios pasos a la vez, y si apuntas a uno ves la imagen como estaba entonces. Cada icono de la cola tiene su propia lista, que se mantiene hasta que se vacía la cola o se cierra la página. Mientras escribes en un campo, Ctrl+Z deshace allí lo escrito.',

  // ---- Tipos de archivo
  'help.types.01.h2': 'Imágenes que puedes abrir',
  'help.types.02.dl.1.t': 'PNG',
  'help.types.02.dl.1.d': 'La mejor opción: nítido y puede ser transparente.',
  'help.types.02.dl.2.t': 'JPG',
  'help.types.02.dl.2.d': 'Fotos. Un JPG no tiene transparencia, así que su fondo se queda a menos que lo quites.',
  'help.types.02.dl.3.t': 'GIF',
  'help.types.02.dl.3.d': 'Fijo o animado. En uno animado eliges el fotograma a partir del cual se crea el icono.',
  'help.types.02.dl.4.t': 'WebP, BMP, TIFF',
  'help.types.02.dl.4.d': 'Se leen como cualquier otra imagen.',
  'help.types.02.dl.5.t': 'SVG',
  'help.types.02.dl.5.d':
    'Un dibujo: se dibuja de nuevo en cada tamaño, así que es nítido en todos. Un recorte y un giro no se aplican a él. El texto dentro de un SVG no se dibuja, porque la página no tiene tipos de letra para ello (te avisa cuando un SVG tiene texto): convierte primero el texto en contornos.',
  'help.types.02.dl.6.t': 'ICNS',
  'help.types.02.dl.6.d': 'Un icono de macOS existente se puede abrir como imagen.',
  'help.types.03.p':
    'Funciona mejor una imagen cuadrada con fondo transparente. Una imagen de más de 40 megapíxeles (8000 × 5000 píxeles, por ejemplo) se rechaza; la herramienta de línea de comandos admite imágenes mayores ([lo que la página no puede hacer](help:command-line)).',

  'help.types.04.h2': 'Archivos que puedes crear',
  'help.types.05.dl.1.t': '.ico (Windows)',
  'help.types.05.dl.1.d': 'Un archivo con todos los tamaños que elegiste, de 1 a 256 píxeles. Windows, los navegadores y muchos programas lo usan.',
  'help.types.05.dl.2.t': '.icns (macOS)',
  'help.types.05.dl.2.d': 'El formato de icono de macOS. Tiene un conjunto fijo de tamaños: 16, 32, 64, 128, 256, 512 y 1024 píxeles.',
  'help.types.05.dl.3.t': 'ZIP para sitio web',
  'help.types.05.dl.3.d':
    'Un paquete para una página web: favicon.ico (16, 32 y 48 píxeles), favicon-16x16.png, favicon-32x32.png, apple-touch-icon.png (180 píxeles), icon-192.png, icon-512.png, site.webmanifest y head-snippet.html con las líneas para pegar en tu página. Un paquete creado a partir de un SVG incluye además favicon.svg.',
  'help.types.05.dl.4.t': 'ZIP de PNG',
  'help.types.05.dl.4.d': 'Cada tamaño de un icono como un archivo PNG propio, en un ZIP (el botón de archivo junto al botón de descarga).',
  'help.types.05.dl.5.t': 'ZIP de la cola',
  'help.types.05.dl.5.d': 'Todos los iconos de la cola en un ZIP.',

  'help.types.06.h2': 'Abrir un archivo .ico',
  'help.types.07.p':
    'Suelta un archivo .ico en lugar de una imagen para mirar dentro: cada imagen con su tamaño, profundidad de color y formato de almacenamiento, y avisos sobre problemas, por ejemplo un tamaño que Windows querría y que falta. Puedes sacar imágenes sueltas, quedarte solo con algunas o editar la imagen más grande como imagen para crear tamaños nuevos.',

  'help.types.08.h2': 'Muchos archivos de una vez',
  'help.types.09.p':
    'Suelta varias imágenes, o un ZIP, y pasan a la cola (hasta 500 archivos de una vez): las imágenes se convierten en iconos con tus ajustes actuales y los archivos .ico se quedan como están. La cola se descarga como un ZIP.',

  // ---- Privacidad
  'help.privacy.01.h2': 'Tus imágenes se quedan en tu ordenador',
  'help.privacy.02.p':
    'img2ico funciona por completo en tu navegador. Un programa (WebAssembly) que se ejecuta dentro de la página, en tu propio ordenador, crea los iconos. Tus imágenes y los iconos hechos con ellas nunca se suben, y la página no tiene ningún servidor que pudiera recibirlas.',
  'help.privacy.03.h2': 'Qué recuerda la página',
  'help.privacy.04.p': 'Para ahorrarte trabajo, la página guarda unos pocos ajustes pequeños en tu navegador (almacenamiento local), solo en tu ordenador:',
  'help.privacy.05.ul.1': 'tus últimos ajustes: tamaños, margen, esquinas, ajuste, blanco y negro, quitar el fondo, el tipo de archivo y el nombre y los colores del paquete para sitios web (no la imagen, ni el recorte ni el giro)',
  'help.privacy.05.ul.2': 'si la cola guarda los cambios por sí sola',
  'help.privacy.05.ul.3': 'si los ajustes están plegados',
  'help.privacy.05.ul.4': 'modo claro u oscuro, si elegiste uno',
  'help.privacy.05.ul.5': 'si los atajos de teclado están activados',
  'help.privacy.05.ul.6': 'el idioma que elegiste',
  'help.privacy.06.p':
    'La cola y tus imágenes solo existen en la memoria de la página mientras está abierta: al recargar o cerrar la página se vacían.',
  'help.privacy.07.h2': 'Lo que la página no hace',
  'help.privacy.08.ul.1': 'Sin cuenta ni inicio de sesión.',
  'help.privacy.08.ul.2': 'Sin anuncios, sin rastreo y sin analítica.',
  'help.privacy.08.ul.3': 'Sin cookies.',
  'help.privacy.08.ul.4': 'No se envía ninguna imagen ni nombre de archivo a ninguna parte.',
  'help.privacy.09.h2': 'Quien aloja la página',
  'help.privacy.10.p':
    'Como cualquier sitio web, el servidor que entrega la página puede ver que se pidió la página (la dirección, la hora y el nombre de tu navegador). Nunca ve tus imágenes.',
  'help.privacy.11.h2': 'Compruébalo tú mismo',
  'help.privacy.12.p':
    'Abre las herramientas para desarrolladores de tu navegador, mira la pestaña Red y convierte una imagen: no se envía nada. Para borrar lo que la página recuerda, elimina los datos de este sitio en los ajustes de tu navegador. El código fuente es abierto: [img2ico en GitHub](https://github.com/Matek85/img2ico).',

  // ---- Atajos de teclado
  'help.shortcuts.01.p':
    'Teclas sueltas, sin Ctrl, Alt ni Cmd, para que nunca estorben a los atajos del navegador. Funcionan mientras no se está escribiendo texto, y no con un cuadro de diálogo abierto.',
  'help.shortcuts.02.keys': 'Todos los atajos de teclado del editor',
  'help.shortcuts.03.h2': 'Conviene saber',
  'help.shortcuts.04.ul.1':
    'Las letras siguen al teclado en el que escribes. Los dígitos 1 a 3 usan la fila sobre las letras, así que también funcionan en un teclado francés.',
  'help.shortcuts.04.ul.2':
    'Mientras editas la imagen, el teclado numérico coloca el marco como está dispuesto el panel en pantalla (7 arriba a la izquierda, 5 centro, 3 abajo a la derecha), haga lo que haga el bloqueo numérico.',
  'help.shortcuts.04.ul.3':
    'Pulsa [[?]] en el editor para ver esta lista. Allí también puedes desactivar los atajos; la elección se recuerda.',

  // ---- Lo que la página no puede hacer
  'help.cli.01.p':
    'La página y la herramienta de línea de comandos comparten un mismo motor, así que ambas crean los mismos archivos de icono. La herramienta de línea de comandos sirve para lo que una página no puede hacer: trabajar con muchos archivos y carpetas, ejecutarse sin una persona y llevar registros.',
  'help.cli.02.h2': 'Lo que solo hace la línea de comandos',
  'help.cli.03.dl.1.t': 'Carpetas',
  'help.cli.03.dl.1.d':
    'Convertir una carpeta entera, con sus subcarpetas (`--recursive`), solo los archivos que coinciden (`--include`, `--exclude`), conservando la estructura de carpetas (`--keep-structure`) y nombrando los iconos con un patrón (`--name`).',
  'help.cli.03.dl.2.t': 'Mirar antes de hacer',
  'help.cli.03.dl.2.d': '`--what-if` muestra lo que pasaría y no escribe nada.',
  'help.cli.03.dl.3.t': 'Informes',
  'help.cli.03.dl.3.d':
    '`--report` escribe un registro de una ejecución como CSV o JSON, con una línea por archivo y los totales. `--json` imprime los informes de `--inspect` y `--validate` como JSON.',
  'help.cli.03.dl.4.t': 'Archivos de ajustes',
  'help.cli.03.dl.4.d':
    'Guarda tus ajustes en un archivo TOML (`--config`, `--out-toml`) y usa los mismos en cada ejecución, en cada ordenador. La página web guarda y lee el mismo archivo ([los ajustes explicados](help:settings)).',
  'help.cli.03.dl.5.t': 'Comprobaciones en la automatización',
  'help.cli.03.dl.5.d':
    '`--validate` comprueba la estructura de archivos .ico, o de carpetas con ellos, y termina con un código de error cuando alguno no es válido: pensado para la integración continua.',
  'help.cli.03.dl.6.t': 'Lotes cuidadosos',
  'help.cli.03.dl.6.d':
    '`--skip-existing` deja en paz los archivos terminados, `--keep-going` sigue después de un fallo, `--force` sobrescribe y `--delete-source` borra los originales tras un éxito.',
  'help.cli.03.dl.7.t': 'Fondos más difíciles',
  'help.cli.03.dl.7.d':
    '`--seed` añade puntos de partida para quitar el fondo, y `--find` descubre zonas cerradas del color a las que el borde no llega.',
  'help.cli.03.dl.8.t': 'Imágenes grandes y velocidad',
  'help.cli.03.dl.8.d': 'Sin el límite de 40 megapíxeles (`--max-pixels`, 100 millones al principio) y trabajo en varios hilos (`--jobs`).',
  'help.cli.04.h2': 'Cómo conseguirla',
  'help.cli.05.p':
    'El menú **Descargar la CLI** de la barra superior tiene la última versión para Windows, macOS y Linux. Todas las opciones están en [la referencia de comandos en GitHub](https://github.com/Matek85/img2ico#command-reference).',
};
