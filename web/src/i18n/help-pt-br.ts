// Brazilian Portuguese help texts: the same keys and marks as help-en.ts, with the names of the buttons as they read
// in pt-br.ts. Translated by an AI.
import type { helpEn } from './help-en';

export const helpPtBr: Record<keyof typeof helpEn, string> = {
  // ---- Primeiros passos
  'help.start.01.h2': 'Abrir uma imagem',
  'help.start.02.p':
    'Solte uma imagem na página, clique em **Escolher um arquivo** ou cole uma da área de transferência (uma captura de tela, por exemplo) com [[Ctrl]] + [[V]] ([[Cmd]] + [[V]] em um Mac). O img2ico lê imagens PNG, JPG, GIF, WebP, BMP, TIFF, SVG e ICNS; [Tipos de arquivo](help:file-types) diz para que serve cada uma.',
  'help.start.03.p':
    'Várias imagens, ou um ZIP cheio delas, entram em uma fila de uma vez (veja abaixo). Um arquivo .ico que você solta é aberto, em vez disso, para olhar dentro: você pode conferi-lo, tirar imagens dele ou editar a maior imagem dele como imagem.',
  'help.start.04.h2': 'Escolher para que serve o ícone',
  'help.start.05.p':
    'As configurações ao lado da pré-visualização começam com o **Início rápido**. Em **Para que serve?** escolha um conjunto de tamanhos: Aplicativo do Windows, Padrão, Ícone para site, Aplicativo do macOS ou Teste rápido. Em **Estilo** escolha como a imagem fica no quadrado: Simples, Ícone de app arredondado ou Redondo.',
  'help.start.06.p':
    'A pré-visualização muda na hora. As configurações se recolhem em uma barra estreita de ícones para dar espaço à pré-visualização; a seta no topo dela as abre, e o **Editor avançado** tem todas as configurações ([as configurações explicadas](help:settings)).',
  'help.start.07.h2': 'Conferir o resultado',
  'help.start.08.p':
    'A pré-visualização mostra cada tamanho do ícone no tamanho real. A coluna de quadrados à esquerda muda o fundo atrás do ícone: xadrez, claro, escuro, cinza ou uma cor sua (aponte para um para experimentar, clique para guardá-lo). **Ícone** mostra os ícones, **Antes e depois** coloca o original ao lado do resultado sob um divisor que você pode arrastar, e **Pixels** mostra o ícone pixel a pixel com a cor exata de cada um.',
  'help.start.09.p':
    'Os avisos abaixo da pré-visualização dizem quando algo provavelmente vai ficar ruim, por exemplo quando a imagem é menor que um tamanho que você escolheu.',
  'help.start.10.h2': 'Mudar a própria imagem',
  'help.start.11.p':
    '**Editar**, acima da pré-visualização, abre as ferramentas de imagem: usar só uma parte da imagem, girá-la e ver como um estilo redondo a corta. Elas são descritas em [as configurações explicadas](help:settings).',
  'help.start.12.h2': 'Baixar',
  'help.start.13.p':
    'Acima da pré-visualização, escolha o tipo de arquivo: **Windows .ico**, **macOS .icns** ou **ZIP para site**, e clique no botão de download. A barra fica à vista enquanto você rola a página. O botão de arquivo ao lado dele salva cada tamanho como um arquivo PNG, em um ZIP.',
  'help.start.14.h2': 'Criar vários ícones',
  'help.start.15.p':
    '**Adicionar à fila** guarda o ícone e deixa você escolher a próxima imagem. A fila fica ao lado do editor: mude a ordem, abra qualquer ícone para alterá-lo de novo (as alterações são salvas nele sozinhas), compare dois ícones e baixe todos como um ZIP ou um de cada vez. A fila só existe enquanto a página está aberta.',
  'help.start.16.h2': 'Suas configurações são lembradas',
  'help.start.17.p':
    'A página lembra suas últimas configurações neste navegador, para que a próxima imagem comece como você deixou. **Restaurar padrões**, no canto superior direito do editor, volta tudo ao início (pergunta antes). O corte e a rotação pertencem a uma imagem e não são lembrados. [Privacidade](help:privacy) diz o que o navegador guarda.',

  // ---- As configurações explicadas
  'help.settings.01.h2': 'Início rápido',
  'help.settings.02.h3': 'Para que serve?',
  'help.settings.03.dl.1.t': 'Aplicativo do Windows',
  'help.settings.03.dl.1.d':
    'Os dez tamanhos que a Microsoft recomenda (16, 20, 24, 32, 40, 48, 64, 96, 128 e 256 pixels), para que o Windows nunca precise esticar um ícone. Um arquivo .ico.',
  'help.settings.03.dl.2.t': 'Padrão',
  'help.settings.03.dl.2.d': 'Seis tamanhos de 16 a 256 pixels: o que a maioria dos programas inclui. Um arquivo .ico.',
  'help.settings.03.dl.3.t': 'Ícone para site',
  'help.settings.03.dl.3.d':
    'Tudo que um site precisa, em um ZIP: favicon.ico, ícones PNG, um ícone Apple e um manifesto web ([Tipos de arquivo](help:file-types) lista todos).',
  'help.settings.03.dl.4.t': 'Aplicativo do macOS',
  'help.settings.03.dl.4.d':
    'Um arquivo .icns para macOS. Ele tem seu próprio conjunto de tamanhos (de 16 a 1024 pixels), então os tamanhos que você escolhe só decidem o que a pré-visualização mostra.',
  'help.settings.03.dl.5.t': 'Teste rápido',
  'help.settings.03.dl.5.d': 'Só 16 e 32 pixels: um arquivo pequeno, para experimentar.',
  'help.settings.04.h3': 'Estilo',
  'help.settings.05.dl.1.t': 'Simples',
  'help.settings.05.dl.1.d': 'A imagem como ela é, sem margem e sem arredondamento.',
  'help.settings.05.dl.2.t': 'Ícone de app arredondado',
  'help.settings.05.dl.2.d': 'Uma pequena margem e cantos arredondados, como um ícone de aplicativo.',
  'help.settings.05.dl.3.t': 'Redondo',
  'help.settings.05.dl.3.d': 'Preenche um círculo: o centro da imagem é mantido e o resto é cortado.',

  'help.settings.06.h2': 'Tamanhos',
  'help.settings.07.p':
    'Um arquivo .ico contém várias imagens, uma para cada tamanho, em pixels por lado. O Windows usa o tamanho mais próximo que encontra para a barra de tarefas, as barras de título, listas e pré-visualizações grandes, então mais tamanhos significam menos imagens esticadas. No Editor avançado você pode marcar qualquer um de 16, 20, 24, 32, 40, 48, 64, 96, 128 e 256 pixels. Quando a imagem é menor que um tamanho que você escolheu, um aviso diz isso: esse tamanho é ampliado e fica borrado.',

  'help.settings.08.h2': 'Aparência',
  'help.settings.09.dl.1.t': 'Margem',
  'help.settings.09.dl.1.d':
    'Espaço vazio ao redor da imagem, como proporção do ícone (0 a 40%). Um ícone que preenche o quadrado até a borda parece apertado ao lado de outros.',
  'help.settings.09.dl.2.t': 'Cantos arredondados',
  'help.settings.09.dl.2.d': 'O quanto os cantos são redondos, como proporção do lado mais curto (0 a 50%). 50% dá um círculo.',
  'help.settings.09.dl.3.t': 'Ajuste',
  'help.settings.09.dl.3.d':
    '**Mostrar a imagem inteira** mantém tudo e deixa faixas transparentes onde a imagem não é quadrada. **Preencher o quadrado** corta o que sobra, para que o quadrado fique cheio.',
  'help.settings.09.dl.4.t': 'Preto e branco',
  'help.settings.09.dl.4.d': 'Tira toda a cor e mantém o brilho.',
  'help.settings.09.dl.5.t': 'Cortar a margem vazia',
  'help.settings.09.dl.5.d':
    'Corta a borda transparente ao redor do desenho para que ele preencha o ícone (depois do corte, se houver). Útil depois de remover um fundo.',

  'help.settings.10.h2': 'Remover o fundo',
  'help.settings.11.p':
    'Marque **Remover o fundo** para tornar transparente um fundo liso. Por padrão a cor é detectada pela borda da imagem; você também pode escolher a cor ou pegá-la da imagem: clique em um pixel na visão **Pixels**, e as configurações se abrem e piscam onde a cor foi parar.',
  'help.settings.12.dl.1.t': 'Tolerância',
  'help.settings.12.dl.1.d':
    'O quanto uma cor pode diferir da cor de fundo e ainda contar como fundo (0 a 100). Um valor maior remove mais, mas pode comer parte do desenho.',
  'help.settings.12.dl.2.t': 'Borda suave',
  'help.settings.12.dl.2.d': 'O quanto a borda do que é removido é suave (0 a 100). 0 é uma borda dura.',
  'help.settings.13.p':
    'A remoção começa na borda da imagem e avança para dentro, então uma cor que aparece dentro do desenho fica, a menos que toque a área removida.',

  'help.settings.14.h2': 'Editar a imagem',
  'help.settings.15.p':
    '**Editar** (tecla [[C]]) mostra a imagem com uma moldura, e só o que está dentro da moldura é usado. Clique em **Concluído** ou pressione [[Esc]] quando terminar; **Usar a imagem inteira** coloca a moldura de volta ao redor de tudo.',
  'help.settings.16.dl.1.t': 'O cadeado',
  'help.settings.16.dl.1.d':
    'Com o cadeado fechado (no começo) a moldura fica no meio da pré-visualização e você move a imagem sob ela: arraste-a, use o zoom com a roda do mouse ou use as setas com [[+]] e [[-]]. Abra o cadeado ([[Espaço]]) para arrastar, em vez disso, a moldura e suas alças.',
  'help.settings.16.dl.2.t': 'Forma',
  'help.settings.16.dl.2.d': 'Livre, 1:1, 4:3, 3:2 ou 16:9. Uma forma fixa mantém a proporção enquanto você trabalha.',
  'help.settings.16.dl.3.t': 'Posicionar a moldura',
  'help.settings.16.dl.3.d':
    'O painel de 3 × 3 coloca a moldura em uma borda, em um canto ou no meio da imagem e mostra onde ela está. O teclado numérico faz o mesmo.',
  'help.settings.16.dl.4.t': 'Maior moldura, girar a moldura',
  'help.settings.16.dl.4.d':
    '**Maior moldura** deixa a moldura tão grande quanto a imagem permite, com a mesma forma. **Girar a moldura** troca largo por alto.',
  'help.settings.16.dl.5.t': 'Girar a imagem',
  'help.settings.16.dl.5.d':
    'Enquanto o cadeado está fechado, um controle deslizante gira a imagem de −180° a 180° em passos de 1° (um ângulo positivo gira no sentido horário), e botões a giram 90° ou 180°. A imagem gira sob a moldura, e os cantos que a rotação deixa livres ficam transparentes.',
  'help.settings.16.dl.6.t': 'Esquerda, Topo, Largura, Altura',
  'help.settings.16.dl.6.d': 'A moldura em pixels da imagem (depois da rotação). Digite números exatos se precisar.',
  'help.settings.16.dl.7.t': 'Mostrar a forma',
  'help.settings.16.dl.7.d':
    'Com o estilo Ícone de app arredondado ou Redondo, os cantos que o estilo corta ficam escurecidos dentro da moldura, para você ver como o ícone vai ficar. Você pode desligar isso.',
  'help.settings.17.p':
    'O corte e a rotação pertencem à imagem que você tem à frente; não são lembrados para a próxima. Todas as teclas dessas ferramentas estão em [atalhos de teclado](help:keyboard-shortcuts).',

  'help.settings.18.h2': 'GIFs animados',
  'help.settings.19.p':
    'Um GIF animado mostra um pequeno player acima da pré-visualização: reproduzir, pausar, passar pelos quadros e parar no que você quer. O ícone é criado a partir do quadro em que você para.',

  'help.settings.20.h2': 'O pacote para sites',
  'help.settings.21.p':
    'Com **ZIP para site**, a engrenagem abre as configurações do pacote: o nome do site (exibido quando o site é adicionado a uma tela inicial), a cor do tema e o fundo do ícone da Apple (um iPhone preenche áreas transparentes com preto, então o ícone da Apple é colocado sobre esta cor).',

  'help.settings.22.h2': 'A coluna de configurações',
  'help.settings.23.p':
    'A coluna se recolhe em uma barra de ícones para que a pré-visualização possa ser larga; a seta no topo dela a abre. Ao iniciar as ferramentas de imagem ela se recolhe sozinha e abre de novo quando você termina. **Restaurar padrões** pergunta antes e depois volta tamanhos, aparência, fundo, corte e rotação ao início.',

  // ---- Tipos de arquivo
  'help.types.01.h2': 'Imagens que você pode abrir',
  'help.types.02.dl.1.t': 'PNG',
  'help.types.02.dl.1.d': 'A melhor escolha: nítido, e pode ser transparente.',
  'help.types.02.dl.2.t': 'JPG',
  'help.types.02.dl.2.d': 'Fotos. Um JPG não tem transparência, então o fundo dele fica a menos que você o remova.',
  'help.types.02.dl.3.t': 'GIF',
  'help.types.02.dl.3.d': 'Parado ou animado. Em um animado você escolhe o quadro a partir do qual o ícone é criado.',
  'help.types.02.dl.4.t': 'WebP, BMP, TIFF',
  'help.types.02.dl.4.d': 'Lidos como qualquer outra imagem.',
  'help.types.02.dl.5.t': 'SVG',
  'help.types.02.dl.5.d':
    'Um desenho: ele é desenhado de novo em cada tamanho, então é nítido em todos. Corte e rotação não se aplicam a ele. O texto dentro de um SVG ainda não é desenhado, porque a página não tem fontes para isso: transforme o texto em contornos primeiro.',
  'help.types.02.dl.6.t': 'ICNS',
  'help.types.02.dl.6.d': 'Um ícone do macOS existente pode ser aberto como imagem.',
  'help.types.03.p':
    'Uma imagem quadrada com fundo transparente funciona melhor. Uma imagem de mais de 40 megapixels (8000 × 5000 pixels, por exemplo) é recusada; a ferramenta de linha de comando aceita imagens maiores ([o que a página não faz](help:command-line)).',

  'help.types.04.h2': 'Arquivos que você pode criar',
  'help.types.05.dl.1.t': '.ico (Windows)',
  'help.types.05.dl.1.d': 'Um arquivo com todos os tamanhos que você escolheu, de 1 a 256 pixels. Windows, navegadores e muitos programas o usam.',
  'help.types.05.dl.2.t': '.icns (macOS)',
  'help.types.05.dl.2.d': 'O formato de ícone do macOS. Ele tem um conjunto fixo de tamanhos: 16, 32, 64, 128, 256, 512 e 1024 pixels.',
  'help.types.05.dl.3.t': 'ZIP para site',
  'help.types.05.dl.3.d':
    'Um pacote para uma página web: favicon.ico (16, 32 e 48 pixels), favicon-16x16.png, favicon-32x32.png, apple-touch-icon.png (180 pixels), icon-192.png, icon-512.png, site.webmanifest e head-snippet.html com as linhas para colar na sua página. Um pacote feito a partir de um SVG tem também favicon.svg.',
  'help.types.05.dl.4.t': 'ZIP de PNG',
  'help.types.05.dl.4.d': 'Cada tamanho de um ícone como um arquivo PNG próprio, em um ZIP (o botão de arquivo ao lado do botão de download).',
  'help.types.05.dl.5.t': 'ZIP da fila',
  'help.types.05.dl.5.d': 'Todos os ícones da fila em um ZIP.',

  'help.types.06.h2': 'Abrir um arquivo .ico',
  'help.types.07.p':
    'Solte um arquivo .ico em vez de uma imagem para olhar dentro dele: cada imagem com tamanho, profundidade de cor e formato de armazenamento, e avisos sobre problemas, por exemplo um tamanho que o Windows gostaria de ter e que falta. Você pode tirar imagens avulsas, manter só algumas ou editar a maior imagem como imagem para criar novos tamanhos.',

  'help.types.08.h2': 'Muitos arquivos de uma vez',
  'help.types.09.p':
    'Solte várias imagens, ou um ZIP, e elas entram na fila (até 500 arquivos de uma vez): imagens viram ícones com as suas configurações atuais, e arquivos .ico ficam como estão. A fila é baixada como um ZIP.',

  // ---- Privacidade
  'help.privacy.01.h2': 'Suas imagens ficam no seu computador',
  'help.privacy.02.p':
    'O img2ico funciona inteiramente no seu navegador. Um programa (WebAssembly) que roda dentro da página, no seu próprio computador, cria os ícones. Suas imagens e os ícones feitos com elas nunca são enviados, e a página não tem nenhum servidor que pudesse recebê-las.',
  'help.privacy.03.h2': 'O que a página lembra',
  'help.privacy.04.p': 'Para poupar seu trabalho, a página guarda algumas pequenas configurações no seu navegador (armazenamento local), só no seu computador:',
  'help.privacy.05.ul.1': 'suas últimas configurações: tamanhos, margem, cantos, ajuste, preto e branco, remover o fundo, o tipo de arquivo e o nome e as cores do pacote para sites (não a imagem, nem o corte ou a rotação)',
  'help.privacy.05.ul.2': 'se a fila salva as alterações sozinha',
  'help.privacy.05.ul.3': 'se as configurações estão recolhidas',
  'help.privacy.05.ul.4': 'modo claro ou escuro, se você escolheu um',
  'help.privacy.05.ul.5': 'se os atalhos de teclado estão ligados',
  'help.privacy.06.p':
    'A fila e suas imagens só existem na memória da página enquanto ela está aberta: recarregar ou fechar a página as esvazia.',
  'help.privacy.07.h2': 'O que a página não faz',
  'help.privacy.08.ul.1': 'Sem conta e sem login.',
  'help.privacy.08.ul.2': 'Sem anúncios, sem rastreamento e sem análise de uso.',
  'help.privacy.08.ul.3': 'Sem cookies.',
  'help.privacy.08.ul.4': 'Nenhuma imagem e nenhum nome de arquivo é enviado a lugar nenhum.',
  'help.privacy.09.h2': 'Quem hospeda a página',
  'help.privacy.10.p':
    'Como qualquer site, o servidor que entrega a página pode ver que a página foi pedida (o endereço, a hora e o nome do seu navegador). Ele nunca vê suas imagens.',
  'help.privacy.11.h2': 'Confira você mesmo',
  'help.privacy.12.p':
    'Abra as ferramentas de desenvolvedor do seu navegador, olhe a aba Rede e converta uma imagem: nada é enviado. Para apagar o que a página lembra, limpe os dados deste site nas configurações do seu navegador. O código-fonte é aberto: [img2ico no GitHub](https://github.com/Matek85/img2ico).',

  // ---- Atalhos de teclado
  'help.shortcuts.01.p':
    'Teclas avulsas, sem Ctrl, Alt ou Cmd, para que nunca atrapalhem os atalhos do navegador. Funcionam enquanto nenhum texto está sendo digitado, e não com uma caixa de diálogo aberta.',
  'help.shortcuts.02.keys': 'Todos os atalhos de teclado do editor',
  'help.shortcuts.03.h2': 'Bom saber',
  'help.shortcuts.04.ul.1':
    'As letras seguem o teclado em que você digita. Os dígitos 1 a 3 usam a fileira acima das letras, então funcionam também em um teclado francês.',
  'help.shortcuts.04.ul.2':
    'Ao editar a imagem, o teclado numérico posiciona a moldura do jeito que o painel na tela está disposto (7 canto superior esquerdo, 5 centro, 3 canto inferior direito), não importa o que o NumLock faça.',
  'help.shortcuts.04.ul.3':
    'Pressione [[?]] no editor para ver esta lista. Lá você também pode desligar os atalhos; a escolha é lembrada.',

  // ---- O que a página não faz
  'help.cli.01.p':
    'A página e a ferramenta de linha de comando compartilham um motor, então ambas criam os mesmos arquivos de ícone. A ferramenta de linha de comando serve para o que uma página não faz: trabalhar com muitos arquivos e pastas, rodar sem uma pessoa e manter registros.',
  'help.cli.02.h2': 'O que só a linha de comando faz',
  'help.cli.03.dl.1.t': 'Pastas',
  'help.cli.03.dl.1.d':
    'Converter uma pasta inteira, com as subpastas (`--recursive`), só os arquivos que combinam (`--include`, `--exclude`), mantendo a estrutura de pastas (`--keep-structure`) e dando nome aos ícones por um padrão (`--name`).',
  'help.cli.03.dl.2.t': 'Ver antes de fazer',
  'help.cli.03.dl.2.d': '`--what-if` mostra o que aconteceria e não grava nada.',
  'help.cli.03.dl.3.t': 'Relatórios',
  'help.cli.03.dl.3.d':
    '`--report` grava um registro de uma execução como CSV ou JSON, com uma linha por arquivo e os totais. `--json` imprime os relatórios de `--inspect` e `--validate` como JSON.',
  'help.cli.03.dl.4.t': 'Arquivos de configuração',
  'help.cli.03.dl.4.d':
    'Guarde suas configurações em um arquivo TOML (`--config`, `--out-toml`) e use as mesmas em toda execução, em todo computador.',
  'help.cli.03.dl.5.t': 'Verificações na automação',
  'help.cli.03.dl.5.d':
    '`--validate` verifica a estrutura de arquivos .ico, ou de pastas com eles, e termina com um código de erro quando um é inválido: feito para integração contínua.',
  'help.cli.03.dl.6.t': 'Lotes cuidadosos',
  'help.cli.03.dl.6.d':
    '`--skip-existing` deixa em paz os arquivos prontos, `--keep-going` continua depois de uma falha, `--force` sobrescreve e `--delete-source` remove os originais depois de um sucesso.',
  'help.cli.03.dl.7.t': 'Fundos mais difíceis',
  'help.cli.03.dl.7.d':
    '`--seed` adiciona pontos de partida para a remoção do fundo, e `--find` descobre áreas fechadas da cor que a borda não alcança.',
  'help.cli.03.dl.8.t': 'Imagens grandes e velocidade',
  'help.cli.03.dl.8.d': 'Sem o limite de 40 megapixels (`--max-pixels`, 100 milhões no início) e trabalho em várias threads (`--jobs`).',
  'help.cli.04.h2': 'Como obter',
  'help.cli.05.p':
    'O menu **Baixar a CLI** na barra superior tem a versão mais recente para Windows, macOS e Linux. Todas as opções estão em [a referência de comandos no GitHub](https://github.com/Matek85/img2ico#command-reference).',
};
