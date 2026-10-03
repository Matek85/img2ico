// The texts of the help pages. A page is the blocks of its keys, in the order of the numbers:
//   help.<page>.<nn>.h2 | h3 | p | note | keys          one text
//   help.<page>.<nn>.ul.<i> | ol.<i>                      the items of a list
//   help.<page>.<nn>.dl.<i>.t and .d                      a term and what it says
// (see helpContent.ts). In a text, **bold**, `code`, [[Key]] (a key), [words](help:page) (a help
// page) and [words](https://...) (another site) are marked up; nothing else is.
// The title and the description of each page are in en.ts, with the other texts that search
// engines show.
export const helpEn = {
  // ---- Getting started
  'help.start.01.h2': 'Open a picture',
  'help.start.02.p':
    'Drop a picture on the page, press **Choose a file**, or paste one from the clipboard (a screenshot, for example) with [[Ctrl]] + [[V]] ([[Cmd]] + [[V]] on a Mac). img2ico reads PNG, JPG, GIF, WebP, BMP, TIFF, SVG and ICNS pictures; [file types](help:file-types) says what each is good for.',
  'help.start.03.p':
    'Several pictures, or a ZIP full of them, go into a queue at once (see below). An .ico file that you drop is opened for looking inside instead: you can check it, take images out of it, or edit its largest image as a picture.',
  'help.start.04.h2': 'Choose what the icon is for',
  'help.start.05.p':
    'The settings beside the preview start with a **Quick start**. Under **What is it for?** pick a set of sizes: Windows app, Standard, Website icon, macOS app or Quick test. Under **Style** pick how the picture sits in its square: Plain, Rounded app icon or Round.',
  'help.start.06.p':
    'The preview changes at once. The settings fold away to a slim rail of icons to leave room for the preview; the arrow at its top opens them, and the **Advanced editor** has every setting ([the settings explained](help:settings)).',
  'help.start.07.h2': 'Check the result',
  'help.start.08.p':
    'The preview shows every size of the icon at its real size. The column of squares at its left changes the background behind the icon: checkered, light, dark, gray or a color of your own (point at one to try it, click to keep it). **Icon** shows the icons, **Before and after** lays the original beside the result under a divider you can drag, and **Pixels** shows the icon pixel by pixel with the exact color of each.',
  'help.start.09.p':
    'Notes under the preview tell you when something is likely to look bad, for example when the picture is smaller than a size you chose.',
  'help.start.10.h2': 'Change the picture itself',
  'help.start.11.p':
    '**Edit**, above the preview, opens the picture tools: use only a part of the picture, turn it, and see how a round style cuts it. They are described in [the settings explained](help:settings).',
  'help.start.12.h2': 'Download',
  'help.start.13.p':
    'Above the preview, choose the file type: **Windows .ico**, **macOS .icns** or **Website ZIP**, then press the download button. The bar stays in view while you scroll. The archive button next to it saves every size as a PNG file, in a ZIP.',
  'help.start.14.h2': 'Make several icons',
  'help.start.15.p':
    '**Add to queue** keeps the icon and lets you choose the next picture. The queue sits beside the editor: change the order (drag a row by its grip, or use the arrow buttons), open any icon to change it again (the changes are saved to it by themselves), compare two icons, and download them all as one ZIP or one at a time. While you edit an icon of the queue, **Use for all icons** makes the other icons again with the sizes, the look or the background removal of this one. The queue exists only while the page is open.',
  'help.start.16.h2': 'Your settings are remembered',
  'help.start.17.p':
    'The page remembers your last settings in this browser, so the next picture starts the way you left it. **Reset to defaults**, at the top right of the editor, puts everything back (it asks first). The crop and the turn belong to one picture and are not remembered. [Privacy](help:privacy) says what the browser keeps.',
  'help.start.18.h2': 'Language and light or dark mode',
  'help.start.19.p':
    'The **Language** menu in the top bar shows the page in English, German, Spanish, Brazilian Portuguese or French. The first time, the page chooses the language of your browser; your choice is remembered. Some translations were made by an AI, and the page says so at the top. The half-filled circle next to it switches between light and dark mode; until you use it, the page follows your system.',

  // ---- The settings explained
  'help.settings.01.h2': 'Quick start',
  'help.settings.02.h3': 'What is it for?',
  'help.settings.03.dl.1.t': 'Windows app',
  'help.settings.03.dl.1.d':
    'The ten sizes Microsoft recommends (16, 20, 24, 32, 40, 48, 64, 96, 128 and 256 pixels), so Windows never has to stretch an icon. An .ico file.',
  'help.settings.03.dl.2.t': 'Standard',
  'help.settings.03.dl.2.d': 'Six sizes from 16 to 256 pixels: what most programs ship. An .ico file.',
  'help.settings.03.dl.3.t': 'Website icon',
  'help.settings.03.dl.3.d':
    'Everything a website needs, in a ZIP: favicon.ico, PNG icons, an Apple icon and a web manifest ([file types](help:file-types) lists them).',
  'help.settings.03.dl.4.t': 'macOS app',
  'help.settings.03.dl.4.d':
    'An .icns file for macOS. It has its own set of sizes (16 to 1024 pixels), so the sizes you choose only decide what the preview shows.',
  'help.settings.03.dl.5.t': 'Quick test',
  'help.settings.03.dl.5.d': 'Only 16 and 32 pixels: a small file, for trying things out.',
  'help.settings.04.h3': 'Style',
  'help.settings.05.dl.1.t': 'Plain',
  'help.settings.05.dl.1.d': 'The picture as it is, with no margin and no rounding.',
  'help.settings.05.dl.2.t': 'Rounded app icon',
  'help.settings.05.dl.2.d': 'A small margin and rounded corners, like an app icon.',
  'help.settings.05.dl.3.t': 'Round',
  'help.settings.05.dl.3.d': 'Fills a circle: the middle of the picture is kept and the rest is cut away.',

  'help.settings.06.h2': 'Sizes',
  'help.settings.07.p':
    'An .ico file holds several images, one for each size, in pixels on a side. Windows uses the closest size it finds for the task bar, title bars, lists and large previews, so more sizes mean fewer stretched pictures. In the Advanced editor you can tick any of 16, 20, 24, 32, 40, 48, 64, 96, 128 and 256 pixels. When the picture is smaller than a size you chose, a note says so: that size is enlarged and looks soft.',

  'help.settings.08.h2': 'Look',
  'help.settings.09.dl.1.t': 'Margin',
  'help.settings.09.dl.1.d':
    'Empty space around the picture, as a share of the icon (0 to 40 %). An icon that fills its square to the edge looks cramped beside others.',
  'help.settings.09.dl.2.t': 'Rounded corners',
  'help.settings.09.dl.2.d': 'How round the corners are, as a share of the shorter side (0 to 50 %). 50 % makes a circle.',
  'help.settings.09.dl.3.t': 'Fit',
  'help.settings.09.dl.3.d':
    '**Show the whole picture** keeps all of it and leaves transparent bands where the picture is not square. **Fill the square** cuts what overhangs, so the square is full.',
  'help.settings.09.dl.4.t': 'Black and white',
  'help.settings.09.dl.4.d': 'Takes away all color and keeps the brightness.',
  'help.settings.09.dl.5.t': 'Cut away the empty margin',
  'help.settings.09.dl.5.d':
    'Cuts off the transparent border around the artwork so that it fills the icon (after the crop, if there is one). Useful after removing a background.',

  'help.settings.10.h2': 'Remove the background',
  'help.settings.11.p':
    'Tick **Remove the background** to make a plain background transparent. By default the color is detected from the border of the picture; you can also choose the color yourself, or pick it from the picture: click a pixel in the **Pixels** view, and the settings open and flash where the color went.',
  'help.settings.12.dl.1.t': 'Tolerance',
  'help.settings.12.dl.1.d':
    'How different a color may be from the background color and still count as background (0 to 100). A higher value takes more away, but can eat into the artwork.',
  'help.settings.12.dl.2.t': 'Soft edge',
  'help.settings.12.dl.2.d': 'How soft the edge of what is removed is (0 to 100). 0 is a hard edge.',
  'help.settings.13.p':
    'The removal starts at the border of the picture and works inward, so a color that appears inside the artwork stays unless it touches the removed area.',

  'help.settings.14.h2': 'Edit the picture',
  'help.settings.15.p':
    '**Edit** (key [[C]]) shows the picture with a frame, and only what is inside the frame is used. Press **Done** or [[Esc]] when you are finished; **Use the whole picture** puts the frame back around everything.',
  'help.settings.16.dl.1.t': 'The lock',
  'help.settings.16.dl.1.d':
    'With the lock closed (the start) the frame stays in the middle of the preview and you move the picture beneath it: drag it, zoom with the mouse wheel, or use the arrow keys with [[+]] and [[-]]. Open the lock ([[Space]]) to drag the frame and its handles instead.',
  'help.settings.16.dl.2.t': 'Shape',
  'help.settings.16.dl.2.d': 'Free, 1:1, 4:3, 3:2 or 16:9. A fixed shape keeps its ratio while you work.',
  'help.settings.16.dl.3.t': 'Place the frame',
  'help.settings.16.dl.3.d':
    'The 3 × 3 pad puts the frame at an edge, a corner or the middle of the picture, and shows where it is. The number pad does the same.',
  'help.settings.16.dl.4.t': 'Largest frame, turn the frame',
  'help.settings.16.dl.4.d':
    '**Largest frame** makes the frame as big as the picture allows, in the same shape. **Turn the frame** swaps wide and tall.',
  'help.settings.16.dl.5.t': 'Turn and mirror the picture',
  'help.settings.16.dl.5.d':
    'While the lock is closed, a slider turns the picture from −180° to 180° in steps of 1° (a positive angle turns it clockwise), and buttons turn it by 90° or 180°. The picture turns beneath the frame, and the corners the turn leaves are transparent. Two buttons mirror the picture, left to right and top to bottom; it is mirrored first, then turned.',
  'help.settings.16.dl.6.t': 'Left, Top, Width, Height',
  'help.settings.16.dl.6.d': 'The frame in pixels of the picture (after the turn). Type exact numbers if you need them.',
  'help.settings.16.dl.7.t': 'Show the shape',
  'help.settings.16.dl.7.d':
    'With the Rounded app icon or Round style, the corners the style cuts away are shaded inside the frame, so you see how the icon will look. You can switch it off.',
  'help.settings.17.p':
    'The crop and the turn belong to the picture in front of you; they are not remembered for the next one. Every key of these tools is in [keyboard shortcuts](help:keyboard-shortcuts).',

  'help.settings.18.h2': 'Animated GIFs',
  'help.settings.19.p':
    'An animated GIF shows a small player above the preview: play, pause, step through the frames and stop on the one you want. The icon is made from the frame you stop at.',

  'help.settings.20.h2': 'The website package',
  'help.settings.21.p':
    'With **Website ZIP**, the gear opens the settings of the package: the site name (shown when the site is added to a home screen), the theme color, and the background of the Apple icon (an iPhone fills transparent areas with black, so the Apple icon is laid on this color).',

  'help.settings.22.h2': 'The settings column',
  'help.settings.23.p':
    'The column folds away to a rail of icons so the preview can be wide; the arrow at its top opens it. Starting the picture tools folds it for you, and it opens again when you are done. **Reset to defaults** asks first, then puts sizes, look, background, crop and turn back to the start.',
  'help.settings.24.h3': 'Settings as a file',
  'help.settings.25.p':
    '**Export settings**, beside Reset to defaults, saves the sizes, margin, corners, fit, black and white, the background removal and the file type as a small text file named `img2ico.toml`. It is the same kind of file the command line reads, so the export is compatible with it: `img2ico --config img2ico.toml logo.png`, or leave the file in your folder and the command line finds it by itself. The other way round, **Import settings** takes a file saved here or written for the command line, shows the settings, and says what it did not take over (the options of the command line that have no use here). A line the page does not know, or a value that is not allowed, stops the import with a list, and nothing is changed. The mirror, the turn, the crop and the frame of a GIF are in the file when you used them, so a file saved here gives the same icon when you import it again, and on the command line (which knows `--flip-horizontal`, `--flip-vertical`, `--rotate`, `--crop` and `--gif-frame`). A crop that does not fit the picture you import into is left out, and a note says so.',
  'help.settings.26.h3': 'Undo and redo',
  'help.settings.27.p':
    'The two arrows at the top right of the editor, **Ctrl+Z** (Cmd+Z on a Mac) and **Ctrl+Shift+Z** or **Ctrl+Y** take back and bring back the last changes of the settings, the crop frame, the turn and the mirror included. A run of small changes, such as dragging a slider, counts as one. **Reset to defaults** and **Import settings** can be undone too. Under the editor, a list of changes shows every step. Click one to go back (or forward) several steps at once, or point at one to see the picture as it was then. Every icon of the queue has its own list, which stays until the queue is emptied or the page is closed. While you type in a field, Ctrl+Z undoes the typing there.',

  // ---- File types
  'help.types.01.h2': 'Pictures you can open',
  'help.types.02.dl.1.t': 'PNG',
  'help.types.02.dl.1.d': 'The best choice: sharp, and it can be transparent.',
  'help.types.02.dl.2.t': 'JPG',
  'help.types.02.dl.2.d': 'Photos. A JPG has no transparency, so its background stays unless you remove it.',
  'help.types.02.dl.3.t': 'GIF',
  'help.types.02.dl.3.d': 'Still or animated. For an animated one you choose the frame the icon is made from.',
  'help.types.02.dl.4.t': 'WebP, BMP, TIFF',
  'help.types.02.dl.4.d': 'Read like any other picture.',
  'help.types.02.dl.5.t': 'SVG',
  'help.types.02.dl.5.d':
    'A drawing: it is drawn anew at every size, so it is sharp at all of them. A crop and a turn do not apply to it. Text inside an SVG is not drawn, because the page has no fonts for it (it warns you when an SVG has text): turn text into outlines first.',
  'help.types.02.dl.6.t': 'ICNS',
  'help.types.02.dl.6.d': 'An existing macOS icon can be opened as a picture.',
  'help.types.03.p':
    'A square picture with a transparent background works best. A picture of more than 40 megapixels (8000 × 5000 pixels, say) is refused; the command-line tool takes bigger ones ([what the page cannot do](help:command-line)).',

  'help.types.04.h2': 'Files you can make',
  'help.types.05.dl.1.t': '.ico (Windows)',
  'help.types.05.dl.1.d': 'One file with all the sizes you chose, from 1 to 256 pixels. Windows, browsers and many programs use it.',
  'help.types.05.dl.2.t': '.icns (macOS)',
  'help.types.05.dl.2.d': 'The macOS icon format. It has a fixed set of sizes: 16, 32, 64, 128, 256, 512 and 1024 pixels.',
  'help.types.05.dl.3.t': 'Website ZIP',
  'help.types.05.dl.3.d':
    'A package for a web page: favicon.ico (16, 32 and 48 pixels), favicon-16x16.png, favicon-32x32.png, apple-touch-icon.png (180 pixels), icon-192.png, icon-512.png, site.webmanifest, and head-snippet.html with the lines to paste into your page. A package made from an SVG has favicon.svg as well.',
  'help.types.05.dl.4.t': 'PNG ZIP',
  'help.types.05.dl.4.d': 'Every size of an icon as a PNG file of its own, in a ZIP (the archive button beside the download button).',
  'help.types.05.dl.5.t': 'Queue ZIP',
  'help.types.05.dl.5.d': 'All the icons of the queue in one ZIP.',

  'help.types.06.h2': 'Opening an .ico file',
  'help.types.07.p':
    'Drop an .ico file instead of a picture to look inside it: every image with its size, color depth and storage format, and notes about problems, for example a size that Windows would want and that is missing. You can take single images out of it, keep only some of them, or edit the largest image as a picture to make new sizes.',

  'help.types.08.h2': 'Many files at once',
  'help.types.09.p':
    'Drop several pictures, or a ZIP, and they go into the queue (up to 500 files at once): pictures become icons with your current settings, and .ico files stay as they are. The queue downloads as one ZIP.',

  // ---- Privacy
  'help.privacy.01.h2': 'Your pictures stay on your computer',
  'help.privacy.02.p':
    'img2ico works entirely in your browser. A program (WebAssembly) that runs inside the page, on your own computer, makes the icons. Your pictures and the icons made from them are never uploaded, and the page has no server that could receive them.',
  'help.privacy.03.h2': 'What the page remembers',
  'help.privacy.04.p': 'To save you work, the page keeps a few small settings in your browser (local storage), on your computer only:',
  'help.privacy.05.ul.1': 'your last settings: sizes, margin, corners, fit, black and white, background removal, the file type and the name and colors of the website package (not the picture, and not the crop or the turn)',
  'help.privacy.05.ul.2': 'whether the queue saves changes by itself',
  'help.privacy.05.ul.3': 'whether the settings are folded away',
  'help.privacy.05.ul.4': 'light or dark mode, if you chose one',
  'help.privacy.05.ul.5': 'whether the keyboard shortcuts are on',
  'help.privacy.05.ul.6': 'the language you chose',
  'help.privacy.06.p':
    'The queue and your pictures exist only in the memory of the page while it is open: reloading or closing the page empties them.',
  'help.privacy.07.h2': 'What the page does not do',
  'help.privacy.08.ul.1': 'No account and no sign-in.',
  'help.privacy.08.ul.2': 'No ads, no tracking and no analytics.',
  'help.privacy.08.ul.3': 'No cookies.',
  'help.privacy.08.ul.4': 'No pictures and no file names are sent anywhere.',
  'help.privacy.09.h2': 'Whoever hosts the page',
  'help.privacy.10.p':
    'Like any website, the server that delivers the page can see that the page was asked for (the address, the time and the name of your browser). It never sees your pictures.',
  'help.privacy.11.h2': 'Check it yourself',
  'help.privacy.12.p':
    "Open your browser's developer tools, look at the Network tab and convert a picture: nothing is sent. To remove what the page remembers, clear the data of this site in your browser's settings. The source code is open: [img2ico on GitHub](https://github.com/Matek85/img2ico).",

  // ---- Keyboard shortcuts
  'help.shortcuts.01.p':
    'Single keys, without Ctrl, Alt or Cmd, so they never get in the way of the shortcuts of the browser. They work while no text is being typed, and not while a dialog is open.',
  'help.shortcuts.02.keys': 'All keyboard shortcuts of the editor',
  'help.shortcuts.03.h2': 'Good to know',
  'help.shortcuts.04.ul.1':
    'Letters follow the keyboard you type on. The digits 1 to 3 use the row above the letters, so they work on a French keyboard too.',
  'help.shortcuts.04.ul.2':
    'While you edit the picture, the number pad places the frame the way the pad on the screen is laid out (7 top left, 5 middle, 3 bottom right), whatever NumLock does.',
  'help.shortcuts.04.ul.3':
    'Press [[?]] in the editor for this list. There you can also switch the shortcuts off; the choice is remembered.',

  // ---- What the page cannot do
  'help.cli.01.p':
    'The page and the command-line tool share one engine, so both make the same icon files. The command-line tool is for what a page cannot do: work on many files and folders, run without a person, and keep records.',
  'help.cli.02.h2': 'What only the command line does',
  'help.cli.03.dl.1.t': 'Folders',
  'help.cli.03.dl.1.d':
    'Convert a whole folder, with its subfolders (`--recursive`), only the files that match (`--include`, `--exclude`), keeping the folder structure (`--keep-structure`) and naming the icons by a pattern (`--name`).',
  'help.cli.03.dl.2.t': 'Look before doing',
  'help.cli.03.dl.2.d': '`--what-if` shows what would happen and writes nothing.',
  'help.cli.03.dl.3.t': 'Reports',
  'help.cli.03.dl.3.d':
    '`--report` writes a record of a run as CSV or JSON, with one line per file and the totals. `--json` prints the reports of `--inspect` and `--validate` as JSON.',
  'help.cli.03.dl.4.t': 'Settings files',
  'help.cli.03.dl.4.d':
    'Keep your settings in a TOML file (`--config`, `--out-toml`) and use the same ones for every run, on every computer. The web page saves and reads the same file ([the settings explained](help:settings)).',
  'help.cli.03.dl.5.t': 'Checks in automation',
  'help.cli.03.dl.5.d':
    '`--validate` checks the structure of .ico files, or of folders of them, and ends with an error code when one is invalid: made for continuous integration.',
  'help.cli.03.dl.6.t': 'Careful batches',
  'help.cli.03.dl.6.d':
    '`--skip-existing` leaves finished files alone, `--keep-going` carries on after a failure, `--force` overwrites, and `--delete-source` removes the originals after a success.',
  'help.cli.03.dl.7.t': 'Harder backgrounds',
  'help.cli.03.dl.7.d':
    '`--seed` adds starting points for the background removal, and `--find` discovers enclosed areas of the color that the border cannot reach.',
  'help.cli.03.dl.8.t': 'Big pictures and speed',
  'help.cli.03.dl.8.d': 'No limit of 40 megapixels (`--max-pixels`, 100 million to start with) and work on several threads (`--jobs`).',
  'help.cli.04.h2': 'Get it',
  'help.cli.05.p':
    'The **Download CLI** menu in the top bar has the latest version for Windows, macOS and Linux. Every option is listed in [the command reference on GitHub](https://github.com/Matek85/img2ico#command-reference).',
} as const;
