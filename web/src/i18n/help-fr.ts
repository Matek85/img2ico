// French help texts: the same keys and marks as help-en.ts, with the names of the buttons as they read in fr.ts.
// Translated by an AI.
import type { helpEn } from './help-en.ts';

export const helpFr: Record<keyof typeof helpEn, string> = {
  // ---- Premiers pas
  'help.start.01.h2': 'Ouvrir une image',
  'help.start.02.p':
    'Déposez une image sur la page, cliquez sur **Choisir un fichier** ou collez-en une depuis le presse-papiers (une capture d’écran, par exemple) avec [[Ctrl]] + [[V]] ([[Cmd]] + [[V]] sur un Mac). img2ico lit les images PNG, JPG, GIF, WebP, BMP, TIFF, SVG et ICNS ; [Types de fichiers](help:file-types) dit à quoi sert chacune.',
  'help.start.03.p':
    'Plusieurs images, ou un ZIP qui en est rempli, vont d’un coup dans une file d’attente (voir plus bas). Un fichier .ico que vous déposez est plutôt ouvert pour regarder à l’intérieur : vous pouvez le vérifier, en extraire des images ou modifier sa plus grande image comme image.',
  'help.start.04.h2': 'Choisir l’usage de l’icône',
  'help.start.05.p':
    'Les réglages à côté de l’aperçu commencent par le **Démarrage rapide**. Sous **À quoi sert-il ?** choisissez un jeu de tailles : Application Windows, Standard, Icône de site web, Application macOS ou Test rapide. Sous **Style** choisissez comment l’image se place dans son carré : Simple, Icône d’application arrondie ou Rond.',
  'help.start.06.p':
    'L’aperçu change aussitôt. Les réglages se replient en une fine barre d’icônes pour laisser de la place à l’aperçu ; la flèche en haut de cette barre les ouvre, et l’**Éditeur avancé** contient tous les réglages ([les réglages expliqués](help:settings)).',
  'help.start.07.h2': 'Vérifier le résultat',
  'help.start.08.p':
    'L’aperçu montre chaque taille de l’icône à sa taille réelle. La colonne de carrés à sa gauche change l’arrière-plan derrière l’icône : damier, clair, sombre, gris ou une couleur de votre choix (pointez-en une pour l’essayer, cliquez pour la garder). **Icône** montre les icônes, **Avant et après** place l’original à côté du résultat sous un séparateur que vous pouvez faire glisser, et **Pixels** montre l’icône pixel par pixel avec la couleur exacte de chacun.',
  'help.start.09.p':
    'Les remarques sous l’aperçu vous disent quand quelque chose risque d’être laid, par exemple quand l’image est plus petite qu’une taille que vous avez choisie.',
  'help.start.10.h2': 'Modifier l’image elle-même',
  'help.start.11.p':
    '**Modifier**, au-dessus de l’aperçu, ouvre les outils d’image : n’utiliser qu’une partie de l’image, la pivoter et voir comment un style rond la découpe. Ils sont décrits dans [les réglages expliqués](help:settings).',
  'help.start.12.h2': 'Télécharger',
  'help.start.13.p':
    'Au-dessus de l’aperçu, choisissez le type de fichier : **Windows .ico**, **macOS .icns**, **ZIP pour site web** ou **Image** (l’image en fichier PNG, JPG, WebP, BMP ou TIFF), puis cliquez sur le bouton de téléchargement. La barre reste visible pendant que vous faites défiler la page. Le bouton d’archive à côté enregistre chaque taille en fichier PNG, dans un ZIP.',
  'help.start.14.h2': 'Créer plusieurs icônes',
  'help.start.15.p':
    '**Ajouter à la file d’attente** garde l’icône et vous laisse choisir l’image suivante. La file d’attente se trouve à côté de l’éditeur : changez l’ordre (faites glisser une ligne par sa poignée ou utilisez les boutons fléchés), ouvrez n’importe quelle icône pour la modifier de nouveau (les modifications y sont enregistrées toutes seules), comparez deux icônes et téléchargez-les toutes en un ZIP ou une par une. Pendant que vous modifiez une icône de la file d’attente, **Utiliser pour toutes les icônes** recrée les autres icônes avec les tailles, l’aspect ou la suppression de l’arrière-plan de celle-ci. La file d’attente n’existe que tant que la page est ouverte.',
  'help.start.16.h2': 'Vos réglages sont mémorisés',
  'help.start.17.p':
    'La page mémorise vos derniers réglages dans ce navigateur, pour que l’image suivante commence comme vous l’avez laissé. **Rétablir les valeurs par défaut**, en haut à droite de l’éditeur, remet tout (il demande confirmation). Le recadrage et la rotation appartiennent à une image et ne sont pas mémorisés. [Confidentialité](help:privacy) dit ce que le navigateur conserve.',
  'help.start.18.h2': 'Langue et mode clair ou sombre',
  'help.start.19.p':
    'Le menu **Langue** de la barre du haut affiche la page en anglais, allemand, espagnol, portugais du Brésil ou français. La première fois, la page choisit la langue de votre navigateur ; votre choix est mémorisé. Certaines traductions ont été faites par une IA, et la page l’indique en haut. Le cercle à moitié plein à côté bascule entre le mode clair et le mode sombre ; tant que vous ne l’utilisez pas, la page suit votre système.',

  // ---- Les réglages expliqués
  'help.settings.01.h2': 'Démarrage rapide',
  'help.settings.02.h3': 'À quoi sert-il ?',
  'help.settings.03.dl.1.t': 'Application Windows',
  'help.settings.03.dl.1.d':
    'Les dix tailles recommandées par Microsoft (16, 20, 24, 32, 40, 48, 64, 96, 128 et 256 pixels), pour que Windows n’ait jamais à étirer une icône. Un fichier .ico.',
  'help.settings.03.dl.2.t': 'Standard',
  'help.settings.03.dl.2.d': 'Six tailles de 16 à 256 pixels : ce que livrent la plupart des programmes. Un fichier .ico.',
  'help.settings.03.dl.3.t': 'Icône de site web',
  'help.settings.03.dl.3.d':
    'Tout ce dont un site web a besoin, dans un ZIP : favicon.ico, icônes PNG, une icône Apple et un manifeste web ([Types de fichiers](help:file-types) les énumère).',
  'help.settings.03.dl.4.t': 'Application macOS',
  'help.settings.03.dl.4.d':
    'Un fichier .icns pour macOS. Il a son propre jeu de tailles (de 16 à 1024 pixels), les tailles que vous choisissez ne décident donc que de ce que montre l’aperçu.',
  'help.settings.03.dl.5.t': 'Test rapide',
  'help.settings.03.dl.5.d': 'Seulement 16 et 32 pixels : un petit fichier, pour essayer.',
  'help.settings.04.h3': 'Style',
  'help.settings.05.dl.1.t': 'Simple',
  'help.settings.05.dl.1.d': 'L’image telle quelle, sans marge ni arrondi.',
  'help.settings.05.dl.2.t': 'Icône d’application arrondie',
  'help.settings.05.dl.2.d': 'Une petite marge et des coins arrondis, comme une icône d’application.',
  'help.settings.05.dl.3.t': 'Rond',
  'help.settings.05.dl.3.d': 'Remplit un cercle : le centre de l’image est gardé et le reste est coupé.',

  'help.settings.06.h2': 'Tailles',
  'help.settings.07.p':
    'Un fichier .ico contient plusieurs images, une par taille, en pixels par côté. Windows utilise la taille la plus proche qu’il trouve pour la barre des tâches, les barres de titre, les listes et les grands aperçus ; plus de tailles signifient donc moins d’images étirées. Dans l’Éditeur avancé, vous pouvez cocher n’importe laquelle des tailles 16, 20, 24, 32, 40, 48, 64, 96, 128 et 256 pixels. Quand l’image est plus petite qu’une taille que vous avez choisie, une remarque le dit : cette taille est agrandie et paraît floue.',

  'help.settings.08.h2': 'Aspect',
  'help.settings.09.dl.1.t': 'Marge',
  'help.settings.09.dl.1.d':
    'Espace vide autour de l’image, en proportion de l’icône (0 à 40 %). Une icône qui remplit son carré jusqu’au bord paraît à l’étroit à côté d’autres.',
  'help.settings.09.dl.2.t': 'Coins arrondis',
  'help.settings.09.dl.2.d': 'Le degré d’arrondi des coins, en proportion du côté le plus court (0 à 50 %). 50 % donne un cercle.',
  'help.settings.09.dl.3.t': 'Ajustement',
  'help.settings.09.dl.3.d':
    '**Afficher toute l’image** la garde entière et laisse des bandes transparentes là où l’image n’est pas carrée. **Remplir le carré** coupe ce qui dépasse, pour que le carré soit plein.',
  'help.settings.09.dl.4.t': 'Noir et blanc',
  'help.settings.09.dl.4.d': 'Enlève toute la couleur et garde la luminosité.',
  'help.settings.09.dl.5.t': 'Couper la marge vide',
  'help.settings.09.dl.5.d':
    'Coupe la bordure transparente autour du motif pour qu’il remplisse l’icône (après le recadrage, s’il y en a un). Utile après la suppression d’un arrière-plan.',

  'help.settings.10.h2': 'Supprimer l’arrière-plan',
  'help.settings.11.p':
    'Cochez **Supprimer l’arrière-plan** pour rendre transparent un arrière-plan uni. Par défaut, la couleur est détectée depuis le bord de l’image ; vous pouvez aussi choisir la couleur vous-même, ou la prélever dans l’image : cliquez sur un pixel dans la vue **Pixels**, et les réglages s’ouvrent et clignotent là où la couleur est allée.',
  'help.settings.12.dl.1.t': 'Tolérance',
  'help.settings.12.dl.1.d':
    'Dans quelle mesure une couleur peut différer de la couleur d’arrière-plan et compter quand même comme arrière-plan (0 à 100). Une valeur plus haute enlève plus, mais peut mordre sur le motif.',
  'help.settings.12.dl.2.t': 'Bord adouci',
  'help.settings.12.dl.2.d': 'Le degré de douceur du bord de ce qui est supprimé (0 à 100). 0 est un bord net.',
  'help.settings.13.p':
    'La suppression commence au bord de l’image et avance vers l’intérieur ; une couleur qui apparaît à l’intérieur du motif reste donc, sauf si elle touche la zone supprimée.',

  'help.settings.14.h2': 'Modifier l’image',
  'help.settings.15.p':
    '**Modifier** (touche [[C]]) montre l’image avec un cadre, et seul ce qui se trouve dans le cadre est utilisé. Appuyez sur **Terminé** ou sur [[Échap]] quand vous avez fini ; **Utiliser toute l’image** remet le cadre autour de tout.',
  'help.settings.16.dl.1.t': 'Le cadenas',
  'help.settings.16.dl.1.d':
    'Cadenas fermé (au début), le cadre reste au milieu de l’aperçu et vous déplacez l’image dessous : faites-la glisser, zoomez avec la molette de la souris ou utilisez les flèches avec [[+]] et [[-]]. Ouvrez le cadenas ([[Espace]]) pour faire glisser plutôt le cadre et ses poignées.',
  'help.settings.16.dl.2.t': 'Forme',
  'help.settings.16.dl.2.d': 'Libre, 1:1, 4:3, 3:2 ou 16:9. Une forme fixe garde son rapport pendant que vous travaillez.',
  'help.settings.16.dl.3.t': 'Placer le cadre',
  'help.settings.16.dl.3.d':
    'Le pavé 3 × 3 place le cadre sur un bord, dans un coin ou au milieu de l’image et montre où il se trouve. Le pavé numérique fait de même.',
  'help.settings.16.dl.4.t': 'Plus grand cadre, pivoter le cadre',
  'help.settings.16.dl.4.d':
    '**Plus grand cadre** rend le cadre aussi grand que l’image le permet, de la même forme. **Pivoter le cadre** échange large et haut.',
  'help.settings.16.dl.5.t': 'Faire pivoter et retourner l’image',
  'help.settings.16.dl.5.d':
    'Tant que le cadenas est fermé, un curseur fait pivoter l’image de −180° à 180° par pas de 1° (un angle positif tourne dans le sens horaire), et des boutons la font pivoter de 90° ou 180°. L’image pivote sous le cadre, et les coins que la rotation laisse libres sont transparents. Deux boutons retournent l’image, de gauche à droite et de haut en bas ; elle est d’abord retournée, puis pivotée.',
  'help.settings.16.dl.6.t': 'Gauche, Haut, Largeur, Hauteur',
  'help.settings.16.dl.6.d': 'Le cadre en pixels de l’image (après la rotation). Saisissez des nombres exacts si vous en avez besoin.',
  'help.settings.16.dl.7.t': 'Afficher la forme',
  'help.settings.16.dl.7.d':
    'Avec le style Icône d’application arrondie ou Rond, les coins que le style coupe sont assombris dans le cadre, pour que vous voyiez le rendu de l’icône. Vous pouvez le désactiver.',
  'help.settings.17.p':
    'Le recadrage et la rotation appartiennent à l’image que vous avez sous les yeux ; ils ne sont pas mémorisés pour la suivante. Toutes les touches de ces outils sont dans [raccourcis clavier](help:keyboard-shortcuts).',

  'help.settings.18.h2': 'GIF animés',
  'help.settings.19.p':
    'Un GIF animé affiche un petit lecteur au-dessus de l’aperçu : lecture, pause, défilement des images et arrêt sur celle que vous voulez. L’icône est créée à partir de l’image où vous vous arrêtez.',

  'help.settings.20.h2': 'Le jeu pour site web',
  'help.settings.21.p':
    'Avec **ZIP pour site web**, l’engrenage ouvre les réglages du jeu : le nom du site (affiché quand le site est ajouté à un écran d’accueil), la couleur du thème et l’arrière-plan de l’icône Apple (un iPhone remplit les zones transparentes de noir, l’icône Apple est donc posée sur cette couleur).',

  'help.settings.22.h2': 'La colonne des réglages',
  'help.settings.23.p':
    'La colonne se replie en une barre d’icônes pour que l’aperçu puisse être large ; la flèche en haut l’ouvre. Quand vous lancez les outils d’image, elle se replie toute seule et se rouvre quand vous avez fini. **Rétablir les valeurs par défaut** demande confirmation, puis remet tailles, aspect, arrière-plan, recadrage et rotation au départ.',
  'help.settings.24.h3': 'Les réglages comme fichier',
  'help.settings.25.p':
    '**Exporter les réglages**, à côté de Rétablir les valeurs par défaut, enregistre les tailles, la marge, les coins, l’ajustement, le noir et blanc, la suppression de l’arrière-plan et le type de fichier dans un petit fichier texte nommé `img2ico.toml`. C’est le même genre de fichier que lit la ligne de commande, l’export lui est donc compatible : `img2ico --config img2ico.toml logo.png`, ou laissez le fichier dans votre dossier et la ligne de commande le trouve toute seule. Dans l’autre sens, **Importer des réglages** reprend un fichier enregistré ici ou écrit pour la ligne de commande, montre les réglages et dit ce qu’il n’a pas repris (les options de la ligne de commande qui ne servent à rien ici). Une ligne que la page ne connaît pas, ou une valeur interdite, arrête l’importation avec une liste, et rien n’est modifié. Le retournement, la rotation, le recadrage et l’image d’un GIF sont dans le fichier quand vous les avez utilisés, donc un fichier enregistré ici donne la même icône quand vous l’importez de nouveau, et aussi sur la ligne de commande (qui connaît `--flip-horizontal`, `--flip-vertical`, `--rotate`, `--crop` et `--gif-frame`). Un recadrage qui ne convient pas à l’image dans laquelle vous importez est omis, et une note le dit.',
  'help.settings.26.h3': 'Annuler et rétablir',
  'help.settings.27.p':
    'Les deux flèches en haut à droite de l’éditeur, **Ctrl+Z** (Cmd+Z sur un Mac) et **Ctrl+Maj+Z** ou **Ctrl+Y** annulent et rétablissent les dernières modifications des réglages, y compris le cadre de recadrage, la rotation et le retournement. Une série de petites modifications, comme faire glisser un curseur, compte pour une seule. **Rétablir les valeurs par défaut** et **Importer des réglages** peuvent aussi être annulés. Sous l’éditeur, une liste de modifications montre chaque étape. Un clic revient (ou avance) de plusieurs étapes à la fois, et en survolant une entrée vous voyez l’image telle qu’elle était alors. Chaque icône de la file a sa propre liste, qui reste jusqu’à ce que la file soit vidée ou la page fermée. Pendant que vous saisissez du texte dans un champ, Ctrl+Z y annule la saisie.',

  // ---- Types de fichiers
  'help.types.01.h2': 'Images que vous pouvez ouvrir',
  'help.types.02.dl.1.t': 'PNG',
  'help.types.02.dl.1.d': 'Le meilleur choix : net, et il peut être transparent.',
  'help.types.02.dl.2.t': 'JPG',
  'help.types.02.dl.2.d': 'Des photos. Un JPG n’a pas de transparence, son arrière-plan reste donc à moins que vous ne le supprimiez.',
  'help.types.02.dl.3.t': 'GIF',
  'help.types.02.dl.3.d': 'Fixe ou animé. Pour un GIF animé, vous choisissez l’image à partir de laquelle l’icône est créée.',
  'help.types.02.dl.4.t': 'WebP, BMP, TIFF',
  'help.types.02.dl.4.d': 'Lus comme n’importe quelle autre image.',
  'help.types.02.dl.5.t': 'SVG',
  'help.types.02.dl.5.d':
    'Un dessin : il est redessiné à chaque taille, donc net à toutes. Un recadrage et une rotation ne s’y appliquent pas. Le texte à l’intérieur d’un SVG n’est pas dessiné, car la page n’a pas de polices pour cela (elle vous avertit quand un SVG contient du texte) : convertissez d’abord le texte en tracés.',
  'help.types.02.dl.6.t': 'ICNS',
  'help.types.02.dl.6.d': 'Une icône macOS existante peut être ouverte comme image.',
  'help.types.03.p':
    'Une image carrée à fond transparent donne les meilleurs résultats. Une image de plus de 40 mégapixels (8000 × 5000 pixels, par exemple) est refusée ; l’outil en ligne de commande en accepte de plus grandes ([ce que la page ne sait pas faire](help:command-line)).',

  'help.types.04.h2': 'Fichiers que vous pouvez créer',
  'help.types.05.dl.1.t': '.ico (Windows)',
  'help.types.05.dl.1.d': 'Un fichier avec toutes les tailles choisies, de 1 à 256 pixels. Windows, les navigateurs et de nombreux programmes l’utilisent.',
  'help.types.05.dl.2.t': '.icns (macOS)',
  'help.types.05.dl.2.d': 'Le format d’icône de macOS. Il a un jeu de tailles fixe : 16, 32, 64, 128, 256, 512 et 1024 pixels.',
  'help.types.05.dl.3.t': 'ZIP pour site web',
  'help.types.05.dl.3.d':
    'Un jeu pour une page web : favicon.ico (16, 32 et 48 pixels), favicon-16x16.png, favicon-32x32.png, apple-touch-icon.png (180 pixels), icon-192.png, icon-512.png, site.webmanifest et head-snippet.html avec les lignes à coller dans votre page. Un jeu fait à partir d’un SVG contient aussi favicon.svg.',
  'help.types.05.dl.4.t': 'Fichier image (PNG, JPG, WebP, BMP, TIFF)',
  'help.types.05.dl.4.d':
    'L’image comme un fichier image ordinaire, avec le choix « Image » au-dessus de l’aperçu. **Taille d’origine** est l’image telle que vous l’avez modifiée (recadrée, tournée, retournée, arrière-plan supprimé, en noir et blanc), dans sa propre taille ; la marge et les coins de l’icône ne comptent pas, et un dessin (SVG) sort à la taille qu’il déclare. **Tailles de l’icône** crée une image carrée pour chaque taille cochée, avec la marge et les coins de l’icône ; une taille donne un fichier, plusieurs arrivent dans un ZIP. JPG et BMP n’ont pas de transparence, donc les zones transparentes reçoivent une couleur d’arrière-plan que vous choisissez. Le WebP est enregistré sans perte. GIF et SVG ne sont pas proposés comme type de fichier à enregistrer.',
  'help.types.05.dl.5.t': 'ZIP de PNG',
  'help.types.05.dl.5.d': 'Chaque taille d’une icône en fichier PNG à part, dans un ZIP (le bouton d’archive à côté du bouton de téléchargement).',
  'help.types.05.dl.6.t': 'ZIP de la file d’attente',
  'help.types.05.dl.6.d': 'Toutes les icônes de la file d’attente dans un ZIP.',

  'help.types.06.h2': 'Ouvrir un fichier .ico',
  'help.types.07.p':
    'Déposez un fichier .ico au lieu d’une image pour regarder à l’intérieur : chaque image avec sa taille, sa profondeur de couleur et son format de stockage, et des remarques sur les problèmes, par exemple une taille que Windows aimerait avoir et qui manque. Vous pouvez en extraire des images seules, n’en garder que certaines, ou modifier la plus grande image comme image pour créer de nouvelles tailles.',

  'help.types.08.h2': 'Beaucoup de fichiers d’un coup',
  'help.types.09.p':
    'Déposez plusieurs images, ou un ZIP, et elles vont dans la file d’attente (jusqu’à 500 fichiers d’un coup) : les images deviennent des icônes avec vos réglages actuels, et les fichiers .ico restent tels quels. La file d’attente se télécharge en un ZIP.',

  // ---- Confidentialité
  'help.privacy.01.h2': 'Vos images restent sur votre ordinateur',
  'help.privacy.02.p':
    'img2ico fonctionne entièrement dans votre navigateur. Un programme (WebAssembly) qui s’exécute dans la page, sur votre propre ordinateur, crée les icônes. Vos images et les icônes qui en sont faites ne sont jamais envoyées, et la page n’a aucun serveur qui pourrait les recevoir.',
  'help.privacy.03.h2': 'Ce que la page mémorise',
  'help.privacy.04.p': 'Pour vous épargner du travail, la page garde quelques petits réglages dans votre navigateur (stockage local), sur votre ordinateur seulement :',
  'help.privacy.05.ul.1': 'vos derniers réglages : tailles, marge, coins, ajustement, noir et blanc, suppression de l’arrière-plan, le type de fichier ainsi que le nom et les couleurs du jeu pour site web (pas l’image, ni le recadrage ni la rotation)',
  'help.privacy.05.ul.2': 'si la file d’attente enregistre les modifications toute seule',
  'help.privacy.05.ul.3': 'si les réglages sont repliés',
  'help.privacy.05.ul.4': 'le mode clair ou sombre, si vous en avez choisi un',
  'help.privacy.05.ul.5': 'si les raccourcis clavier sont activés',
  'help.privacy.05.ul.6': 'la langue que vous avez choisie',
  'help.privacy.06.p':
    'La file d’attente et vos images n’existent que dans la mémoire de la page tant qu’elle est ouverte : recharger ou fermer la page les vide.',
  'help.privacy.07.h2': 'Ce que la page ne fait pas',
  'help.privacy.08.ul.1': 'Pas de compte, pas de connexion.',
  'help.privacy.08.ul.2': 'Pas de publicité, pas de suivi et pas de statistiques.',
  'help.privacy.08.ul.3': 'Pas de cookies.',
  'help.privacy.08.ul.4': 'Aucune image et aucun nom de fichier n’est envoyé nulle part.',
  'help.privacy.09.h2': 'Qui héberge la page',
  'help.privacy.10.p':
    'Comme tout site web, le serveur qui délivre la page peut voir qu’elle a été demandée (l’adresse, l’heure et le nom de votre navigateur). Il ne voit jamais vos images.',
  'help.privacy.11.h2': 'Vérifiez par vous-même',
  'help.privacy.12.p':
    'Ouvrez les outils de développement de votre navigateur, regardez l’onglet Réseau et convertissez une image : rien n’est envoyé. Pour effacer ce que la page mémorise, supprimez les données de ce site dans les paramètres de votre navigateur. Le code source est ouvert : [img2ico sur GitHub](https://github.com/Matek85/img2ico).',

  // ---- Raccourcis clavier
  'help.shortcuts.01.p':
    'Des touches seules, sans Ctrl, Alt ni Cmd, pour qu’elles ne gênent jamais les raccourcis du navigateur. Elles fonctionnent tant qu’aucun texte n’est saisi, et pas quand une boîte de dialogue est ouverte.',
  'help.shortcuts.02.keys': 'Tous les raccourcis clavier de l’éditeur',
  'help.shortcuts.03.h2': 'Bon à savoir',
  'help.shortcuts.04.ul.1':
    'Les lettres suivent le clavier sur lequel vous tapez. Les chiffres 1 à 3 utilisent la rangée au-dessus des lettres, ils fonctionnent donc aussi sur un clavier français.',
  'help.shortcuts.04.ul.2':
    'Pendant que vous modifiez l’image, le pavé numérique place le cadre comme le pavé à l’écran est disposé (7 en haut à gauche, 5 au milieu, 3 en bas à droite), quoi que fasse Verr. num.',
  'help.shortcuts.04.ul.3':
    'Appuyez sur [[?]] dans l’éditeur pour voir cette liste. Vous pouvez aussi y désactiver les raccourcis ; le choix est mémorisé.',

  // ---- Ce que la page ne sait pas faire
  'help.cli.01.p':
    'La page et l’outil en ligne de commande partagent un même moteur, donc les deux créent les mêmes fichiers d’icône. L’outil en ligne de commande sert à ce qu’une page ne sait pas faire : travailler sur beaucoup de fichiers et de dossiers, s’exécuter sans personne et tenir des journaux.',
  'help.cli.02.h2': 'Ce que seule la ligne de commande fait',
  'help.cli.03.dl.1.t': 'Dossiers',
  'help.cli.03.dl.1.d':
    'Convertir un dossier entier, avec ses sous-dossiers (`--recursive`), seulement les fichiers qui correspondent (`--include`, `--exclude`), en gardant la structure des dossiers (`--keep-structure`) et en nommant les icônes selon un modèle (`--name`).',
  'help.cli.03.dl.2.t': 'Regarder avant d’agir',
  'help.cli.03.dl.2.d': '`--what-if` montre ce qui se passerait et n’écrit rien.',
  'help.cli.03.dl.3.t': 'Rapports',
  'help.cli.03.dl.3.d':
    '`--report` écrit un journal d’une exécution en CSV ou JSON, avec une ligne par fichier et les totaux. `--json` affiche les rapports de `--inspect` et `--validate` en JSON.',
  'help.cli.03.dl.4.t': 'Fichiers de réglages',
  'help.cli.03.dl.4.d':
    'Gardez vos réglages dans un fichier TOML (`--config`, `--out-toml`) et utilisez les mêmes à chaque exécution, sur chaque ordinateur. La page web enregistre et lit le même fichier ([les réglages expliqués](help:settings)).',
  'help.cli.03.dl.5.t': 'Contrôles dans l’automatisation',
  'help.cli.03.dl.5.d':
    '`--validate` vérifie la structure de fichiers .ico, ou de dossiers qui en contiennent, et se termine par un code d’erreur quand l’un est invalide : conçu pour l’intégration continue.',
  'help.cli.03.dl.6.t': 'Lots prudents',
  'help.cli.03.dl.6.d':
    '`--skip-existing` laisse tranquilles les fichiers terminés, `--keep-going` continue après un échec, `--force` écrase, et `--delete-source` supprime les originaux après un succès.',
  'help.cli.03.dl.7.t': 'Arrière-plans plus difficiles',
  'help.cli.03.dl.7.d':
    '`--seed` ajoute des points de départ pour la suppression de l’arrière-plan, et `--find` découvre des zones enclavées de la couleur que le bord ne peut pas atteindre.',
  'help.cli.03.dl.8.t': 'Grandes images et vitesse',
  'help.cli.03.dl.8.d': 'Pas de limite de 40 mégapixels (`--max-pixels`, 100 millions au départ) et travail sur plusieurs threads (`--jobs`).',
  'help.cli.04.h2': 'Comment l’obtenir',
  'help.cli.05.p':
    'Le menu **Télécharger la CLI** de la barre du haut contient la dernière version pour Windows, macOS et Linux. Toutes les options sont listées dans [la référence des commandes sur GitHub](https://github.com/Matek85/img2ico#command-reference).',
};
