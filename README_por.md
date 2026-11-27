<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### Gravação a laser a partir de uma imagem e corte a partir de um desenho: tramagem, G-code de trama, contornos SVG e DXF e grelha de calibração

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.1.7-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** uma biblioteca Rust e um programa de linha de comandos (`phoenix-laser`) com 1094 testes que correm num computador: converte uma imagem em pontos por tramagem e escreve o G-code de um laser que queima linha a linha com potência dinâmica e sobrecurso, lê desenhos SVG, DXF e LBRN2 e escreve o G-code que corta os seus contornos (os de dentro primeiro, com a largura do feixe, pontes e várias passagens) e grava as suas formas com linhas, com as definições de cada camada num ficheiro, queima texto, códigos QR e códigos de barras, verifica um programa contra os limites da máquina e escreve uma grelha de teste de potência contra avanço. Tem uma janela (compilada com `--features gui`), a moldura de painéis da suite com treze painéis e quatro espaços de trabalho, que abre uma imagem ou um desenho, mostra o percurso do laser, permite alterar as definições e exporta o G-code; a janela não foi vista por uma pessoa num ecrã. O leitor de projetos LBRN2 foi medido com doze projetos reais de um fabricante de lasers (os doze são lidos e a única imagem que contêm é descodificada e gravada por pontos; o sentido do seu eixo y nunca foi confirmado com uma peça real). Também pode enviar um programa para o plugin laser do servidor EH3D e seguir, segurar ou parar o trabalho (`phoenix-laser server`), testado apenas contra um substituto do servidor. **Nada disto correu num laser real**: nada feito com esses projetos nem com outra coisa foi cortado ou queimado numa máquina.

---

> **Estado de segurança:** este programa não comanda nenhuma máquina por si mesmo e não é um sistema de segurança: escreve G-code, verifica-o e pode enviá-lo para o plugin laser do servidor EH3D, que por agora é um simulador. O que produz deve ser verificado por uma pessoa antes de se executar algo numa máquina; nada disto correu em hardware real.


## 📋 Num relance

| | |
|---|---|
| Versão | `0.1.7` |
| Maturidade | Funcional |
| Testes | 1094 |
| Tecnologias | Rust |
| Em hardware real | Não experimentado: nada disto correu numa máquina |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura](#-estrutura-do-repositório) · [Relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Núcleo | 🟡 | converte uma imagem em pontos por tramagem e escreve o G-code de um laser que queima linha a linha com potência dinâmica e sobrecurso, lê desenhos SVG e DXF e corta-os ou grava-os por camada (contornos ordenados, largura do feixe, pontes, passagens, hachura), e uma grelha de teste de potência contra avanço; testado num computador com entradas normais e danificadas, não com uma coleção de ficheiros reais. |
| Linha de comandos | 🟡 | `phoenix-laser` (`engrave`, `calibrate`, `info`, `cut`, `vector`, `layers`) executa as mesmas funções que a biblioteca e imprime o resultado; cada erro volta como mensagem e código de saída diferente de zero. |
| Testes | 🟡 | 1094 testes passam (`cargo test`), com casos normais, casos-limite e entradas danificadas. |
| Segurança | 🟡 | Rust seguro e simples (`forbid(unsafe_code)`): nada entra em pânico com os ficheiros recebidos e um valor mau é um erro com o seu motivo; sem revisão independente. |
| Janela | 🟠 | Escrita (egui, `--features gui`) como a moldura de painéis da suite: abrir ou largar uma imagem ou um desenho SVG, DXF ou LBRN2, ver o percurso do laser (o corte numa cor, a deslocação atenuada), editar a tabela de camadas, os modos de texto, código QR, código de barras, marcas, moldura e foco, uma grelha de calibração, zoom e deslocamento, estimar o tempo, verificar, relatório, enviar para o servidor, exportar; a sua lógica está testada, mas a janela não foi vista por uma pessoa num ecrã. |
| Hardware real | 🔴 | Não experimentado: nada disto foi executado numa máquina. |
| Documentação | 🟡 | README em oito idiomas, o registo de alterações e as duas imagens desenhadas a partir de `docs/`. |

## 🎯 Visão geral

* **A imagem** (`eh3d-image`): PNG (cinzento, cor, transparente, 8 ou 16 bits) e PGM (texto e binário) tornam-se níveis de cinzento, com as partes transparentes como branco e o tamanho limitado para que um ficheiro hostil não possa pedir gigabytes; `resize` faz a média dos píxeis para que um ponto seja tão grande como o feixe.
* **Os pontos** (`src/dither.rs`): um limiar, ou difusão de erro de Floyd-Steinberg ou Atkinson; os testes verificam que a escuridão de um cinzento se conserva (um cinzento médio queima cerca de metade dos pontos) e que o branco não queima nada e o preto queima tudo.
* **O laser** (`src/raster.rs`, `src/calibrate.rs`): uma linha depois da outra, da esquerda para a direita e depois da direita para a esquerda, `M4` com `S0` enquanto viaja e a potência escolhida onde queima, um sobrecurso fixo ou calculado a partir da aceleração (v ao quadrado sobre 2a) e uma grelha de quadrados com uma potência por coluna e um avanço por linha para achar o que convém a um material.
* **O cabeçalho e os números de todo o ficheiro de G-code** (`src/gcode_out.rs`): o ficheiro começa com linhas de comentário que dizem o gerador e a sua versão, as unidades (mm, G21), a imagem de que foi feito com o seu SHA-256 e as ferramentas (o laser e a sua potência); uma grelha de calibração diz que não tem origem. Um número escreve-se de uma só maneira (no máximo três casas decimais, sem `-0`) e um valor que não é finito é recusado em vez de ser escrito como `NaN` ou `inf`.
* **`--dry-run`:** `engrave` calcula o resultado e conta-o (pontos, linhas, passagens, comprimento queimado, tempo) sem escrever o ficheiro de saída, que então pode ser omitido.
* **Os desenhos** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`): um subconjunto seguro de SVG (trajetos com todos os seus comandos, formas, transformações, grupos, unidades e um `use` de um elemento do mesmo ficheiro) e as entidades de um DXF (linhas, polilinhas com arcos, arcos, círculos, elipses, splines, blocos com as suas inserções, unidades) transformam-se em formas em milímetros; os scripts, as imagens, um `use` de outro ficheiro, as entidades declaradas e tudo o que é desconhecido são ignorados ou recusados, o tamanho, a profundidade e o número de elementos (conta o que um `use` traz) e de pontos são limitados pelas definições, um `use` que volta a si mesmo é um erro, e as curvas passam a troços retos dentro de uma tolerância.
* **Corte e gravação** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`): as formas são agrupadas por camada (DXF) ou cor (SVG); cada grupo é cortado (linhas soltas unidas, os contornos que estão dentro de outro primeiro, a menor deslocação, a largura do feixe descontada do lado certo, pontes por cortar, várias passagens com o foco a avançar) ou gravado (formas fechadas preenchidas com linhas num ângulo, pela regra par-ímpar) com as suas próprias definições. `phoenix-laser layers DRAWING --accel A` dá também o tempo de cada camada com a cabeça a travar e a acelerar em cada canto a `A` mm/s2 (e quanto desse tempo o laser queima), contado pela pré-visualização do programa.
* **O ficheiro de definições** (`src/vconfig.rs`, `docs/vector-settings.md`): todo o valor que se afina (os limites dos leitores, a máquina, as definições de cada camada) está num ficheiro de texto simples verificado linha a linha; nada disso está no código.
* **O teste de foco** (`src/calibrate.rs`, `[focus]` no ficheiro de definições): `phoenix-laser focus` queima uma escada de linhas curtas, cada uma com a cabeça a outra altura, e a linha mais nítida mostra onde o feixe está focado; o passo de altura não tem valor por omissão (qual é baixo depende da máquina), a potência obedece ao limite de potência e a cabeça volta à primeira altura. Com `--powers 10,20,30` (até 10 potências) grava em vez disso uma grelha de focagem: as mesmas linhas uma vez para cada potência, lado a lado com uma coluna por potência, para ler que potência grava a linha mais fina à altura do foco; a cabeça vai uma só vez a cada altura e grava todas as colunas dessa fila, e cada potência obedece ao limite de potência.
* **A grelha de teste de um material** (`src/calibrate.rs`, `[calibrate]` no ficheiro de definições): `phoenix-laser calibrate --material NOME --materials FICHEIRO` distribui as potências e as velocidades da grelha em redor do que diz o material da biblioteca, dentro dos limites e sem repetir valores, e `--labels` queima a potência de cada coluna e a velocidade de cada linha ao lado dos quadrados com um tipo de letra de traços, para que a grelha possa ser lida depois no material; um material acima de um limite é recusado, nunca baixado. Com um material os algarismos saem por omissão (`--no-labels` retira-os) e a potência e a velocidade mínimas da escada são as chaves `min_power_percent` e `min_feed_mm_min`.
* **Texto** (`src/text.rs`, `[text]` no ficheiro de definições): `phoenix-laser text OUT.nc --text TEXTO` queima um nome, um número de série ou uma marca com uma fonte de traços do programa (maiúsculas, algarismos, espaço e `. , - + / : ! ? ( ) = _ '`; a minúscula é queimada como maiúscula, uma mudança de linha começa uma fila) e sem ficheiro de fonte. A altura de uma maiúscula, os espaços entre letras e filas, a potência, a velocidade e as passagens são definições; um carácter que a fonte não tem, um texto com mais de `max_chars` caracteres, uma altura acima de `max_height_mm`, uma potência ou uma velocidade acima do limite, ou um trabalho que sairia da cama são recusados, nunca cortados, e o texto nunca é escrito num comentário do programa. As linhas de um texto de várias linhas podem ser alinhadas à esquerda, ao centro ou à direita (`--align`), o texto pode ser gravado como imagem espelhada para o verso de vidro ou acrílico (`--mirror`) e uma gravação que demoraria mais de `max_minutes` é recusada.
* **Código QR** (`src/qr.rs`, `[qr]` no ficheiro de definições): `phoenix-laser qr OUT.nc --text DADOS` grava uns dados como um código QR próprio do programa (versões 1 a 4, de 21 a 33 módulos por lado, em modo de bytes, com os quatro níveis de correção de erros de Reed-Solomon e a máscara de menor penalização), a preencher cada módulo escuro com linhas; o símbolo foi lido de volta com um descodificador independente em todas as versões e níveis, e os testes leem-no de volta com o seu próprio leitor. O lado do módulo, a distância entre as linhas, o nível, a potência, a velocidade, as passagens e os limites são definições; dados acima de `max_bytes`, uma versão acima de `max_version`, um símbolo maior do que `max_size_mm`, um preenchimento de mais de `max_lines` movimentos, uma potência ou uma velocidade acima do limite ou um trabalho fora da mesa são recusados, nunca cortados nem reduzidos, e os dados nunca são escritos num comentário do programa. `--print` mostra o símbolo como texto. Desligado por omissão: `--invert` grava os módulos claros (com `--border-modules`, uma margem gravada à volta), `--mirror` grava a imagem espelhada para o verso de vidro ou acrílico, `--overscan-mm` leva a cabeça até cada linha e além com o laser desligado (recusado se saísse da cama) e uma gravação que demoraria mais de `max_minutes` é recusada.
* **Código de barras** (`src/barcode.rs`, `[barcode]` no ficheiro de definições): `phoenix-laser barcode OUT.nc --text DADOS` grava uns dados como um código de barras Code 128 próprio do programa (os carateres do espaço a `~` no conjunto B, e um texto com um número par de algarismos no conjunto C, com metade da largura, com o seu símbolo de controlo), preenchendo cada barra com linhas ao longo da sua altura; os testes leem-no de volta com o seu próprio leitor, mas não foi lido com um leitor de códigos de barras. A largura da barra mais estreita, a altura, a distância entre as linhas, a potência, a velocidade, as passagens e os limites são definições; um byte que não é um caráter imprimível, mais de `max_chars` carateres, um código mais largo do que `max_width_mm`, um preenchimento de mais de `max_lines` movimentos, uma gravação que demoraria mais de `max_minutes`, uma potência ou uma velocidade acima do limite ou um trabalho fora da mesa são recusados, nunca cortados nem reduzidos, e os dados nunca são escritos num comentário do programa. `--mirror` grava a imagem espelhada para o verso de vidro ou acrílico.
* **Trama cruzada** (`src/cut.rs`, `hatch_cross` numa camada): uma camada `engrave` com `hatch_cross = on` (ou `--cross` em `vector` e `trace`) é preenchida uma segunda vez com linhas em ângulo reto com as primeiras, com o mesmo espaçamento, em cada passagem, e o limite de linhas vale para cada conjunto; a energia de um milímetro quadrado que a lista de camadas dá é duplicada, e a pré-visualização e a verificação do programa veem-no como qualquer outro programa.
* **A moldura do trabalho** (`src/frame.rs`, `[frame]` no ficheiro de definições): `phoenix-laser frame` escreve um programa que percorre o retângulo do trabalho com o laser desligado, a partir de um tamanho ou da caixa do que um programa G-code queima, para ver onde vai cair; uma potência piloto fraca exige `--confirm-laser-on` de cada vez, tem teto de 5 % (`max_pilot_percent` de `[frame]` muda o teto) e obedece ao limite de potência, e uma moldura fora da mesa é recusada.
* **Marcas de posicionamento** (`src/frame.rs`, `[marks]` no ficheiro de definições): `phoenix-laser marks OUT.nc --size W,H` grava com pouca potência um L em cada canto do retângulo do trabalho (nas suas arestas, ou a `margin_mm` por fora) e, com `--centre-cross`, uma cruz ao centro, para poder voltar a pôr o material no mesmo sítio e alinhar um segundo trabalho com o primeiro. Ao contrário da moldura, as marcas são gravadas: a potência e a velocidade obedecem aos limites da máquina, e um braço mais comprido do que metade de um lado do retângulo ou marcas que saíssem da mesa são recusados, nunca cortados. O comprimento dos braços, a margem, a potência, a velocidade e as passagens são definições. `phoenix-laser align PICTURE.pgm --size W,H --px-per-mm S` encontra as marcas numa imagem da mesa tirada de cima e diz quanto se afastam do sítio onde deviam estar (o deslocamento a somar à origem do trabalho e a rotação), recusando uma imagem em que falta uma marca, está longe de mais ou rodada de mais; só foi experimentado com imagens que o `--simulate` desenha, nunca com uma câmara real.
* **A pré-visualização** (`src/preview.rs`): `phoenix-laser preview` lê um programa G-code (o próprio ou de outro), imprime os movimentos, o comprimento queimado e percorrido, a caixa, as potências e um tempo que conta a travagem nos cantos, avisa do que parece perigoso (laser ligado num movimento rápido, sem avanço, potência acima da plena, trabalho fora da mesa, sem fim) e desenha o percurso como SVG; os testes voltam a ler o que a gravação escreve e verificam o mesmo comprimento. Com `--laser-watts W` (a potência do laser a plena potência) diz ainda a energia que o programa grava, a partir da potência de cada movimento como parte de `--smax` durante o tempo em que grava, e a sua potência média.
* **A biblioteca de materiais** (`src/materials.rs`, `examples/materials.cfg`): o que convém a cada material e modo de trabalho escreve-se uma vez num ficheiro de texto (`[material NOME]`: potência, velocidade, passagens, passo de foco, largura do feixe, tracejado, tamanho do ponto, pontilhado e curva de tons) e escolhe-se com `--material NOME --materials FICHEIRO`; as opções da linha de comandos mandam sobre ele e o limite de potência vale sobre os dois. `phoenix-laser materials FICHEIRO` lista a biblioteca.
* **O limite de potência** (`src/safety.rs`, `max_power_percent` em `[machine]`, `--max-power`): um programa cuja potência passa do limite é recusado com um erro que nomeia a camada ou a opção, nunca baixado sem aviso; o limite é 100 % até um ficheiro ou uma opção dizer menos.
* **Perfis de máquina** (`[profile NAME]` no ficheiro de definições, `--profile NAME`): quem tem duas máquinas, ou troca o módulo laser por outro, guarda um perfil para cada uma (o curso da mesa, o limite de potência, o limite de velocidade, o valor de potência máxima e os watts); o perfil escolhido é a máquina, e `--max-speed` acompanha `--max-power`: uma velocidade acima do limite é recusada, nunca baixada.
* **Contornos a partir de uma imagem** (`src/trace.rs`, `[trace]` no ficheiro de definições): `phoenix-laser trace` segue o contorno de um logótipo, de uma silhueta ou de um estêncil em vez de o queimar ponto a ponto; os níveis de cinzento separam-se com um limiar, o bordo é encontrado com marching squares de precisão subpixel, simplificado até nenhum ponto ficar a mais de `simplify_mm` da linha, e as manchas são descartadas; o resultado é uma camada de um desenho, cortada (ou preenchida com linhas) com as mesmas definições e as mesmas verificações, e o tamanho da imagem, os pedaços de bordo, os contornos e os pontos estão todos limitados.
* **Tons, materiais e limites na janela** (`src/window.rs`): as caixas da curva de tons (gama, contraste, brilho, inverter e níveis automáticos) trabalham a imagem com a mesma função da linha de comandos; uma biblioteca de materiais carrega-se a partir de um ficheiro (ou do que `PHOENIX_LASER_MATERIALS` indicar) e escolher um material preenche as caixas de potência, velocidade, ponto, pontilhado, tons e de cada camada de um desenho; os limites de potência e de velocidade também são caixas e um valor acima é recusado, nunca baixado. Os valores iniciais vêm do ficheiro de definições (`[tone]`, `[machine]`).
* **Materiais editados na janela** (`src/window.rs`, `src/materials.rs`): a secção de materiais pode editar, copiar, criar a partir das caixas (potência, velocidade, ponto, sobrepercurso, tramagem e curva de tons) e apagar um material, e criar uma biblioteca nova; uma caixa para cada chave do ficheiro recebe os valores, e ao guardar a secção desse material é reescrita no seu lugar (ou acrescentada no fim) sem tocar nos comentários, nas linhas em branco, na ordem, nos fins de linha nem nos outros materiais. Um valor fora do seu intervalo, um nome vazio, demasiado longo ou já usado, e um ficheiro que não seja UTF-8 ou não seja uma biblioteca válida são recusados, com a chave nomeada e o ficheiro intacto, sem nunca cortar; apagar precisa de um segundo toque, e uma biblioteca nova nunca é escrita sobre um ficheiro que já exista.
* **O teste de focagem na janela** (`src/window.rs`): um modo com as caixas da secção `[focus]` (a altura da primeira linha, o passo, o número e o comprimento das linhas, o espaçamento, a velocidade e a potência); o teste é calculado com a mesma função de `phoenix-laser focus`, a potência e a velocidade obedecem aos limites, não há passo por omissão porque o sentido para baixo depende da máquina, e um aviso de que o teste move o Z fica nas notas. Uma caixa para as potências da grelha (até 10, separadas por vírgulas) faz a janela queimar a grelha de focagem de `focus --powers`: as linhas uma vez para cada potência, uma coluna para cada uma; a caixa da potência única não é usada então, e é recusada uma potência que não seja maior que 0 até 100, que passe o limite ou que seja em número excessivo.
* **Texto na janela** (`src/window.rs`): um modo com uma caixa para as palavras e as caixas da secção `[text]` (a altura de uma maiúscula, a letra mais alta, os espaços entre letras e linhas, a potência, a velocidade, as passagens e o máximo de caracteres); o texto é calculado com a mesma função que `phoenix-laser text`, a partir da origem da máquina, pelo que uma letra que o tipo de letra não tem, um texto acima de um limite, uma potência ou uma velocidade acima do limite da máquina ou um trabalho fora da mesa (quando o ficheiro de definições a indica) é recusado e a linha de estado diz porquê; é escrito como `text.nc` com o cabeçalho da linha de comandos. O alinhamento das linhas, o interruptor de espelho e a duração máxima também estão na janela.
* **Código QR na janela** (`src/window.rs`): um modo com uma caixa para os dados, a escolha do nível de correção de erros e as caixas da secção `[qr]` (o lado do módulo, a distância entre as linhas, a potência, a velocidade, as passagens, o máximo de bytes, a versão e o lado máximos, e o máximo de movimentos de gravação); o símbolo é calculado com a mesma função que `phoenix-laser qr`, a partir da origem da máquina, pelo que dados que não cabem, um símbolo acima de um limite, uma potência ou uma velocidade acima do limite da máquina ou um trabalho fora da mesa (quando o ficheiro de definições a indica) são recusados e a linha de estado diz porquê; é escrito como `qr.nc` com o cabeçalho da linha de comandos. Os interruptores de invertido e de espelho e as caixas da margem, do arranque e da duração máxima também estão na janela.
* **Marcas de posicionamento na janela** (`src/window.rs`, `[marks]` no ficheiro de definições): um modo com caixas para o tamanho do retângulo, o braço de cada L, a margem, a potência, a velocidade e as passagens e um interruptor para a cruz ao centro (um ficheiro de definições preenche as caixas de `[marks]`); as marcas calculam-se com a mesma função de `phoenix-laser marks` a partir da origem da máquina, e é recusado um braço mais comprido que meio lado, uma potência ou uma velocidade acima do limite ou marcas fora da mesa, com a caixa ou o motivo na linha de estado. Escreve-se como `marks.nc` com o cabeçalho habitual.
* **Código de barras Code 128 na janela** (`src/window.rs`, `[barcode]` no ficheiro de definições): um modo com uma caixa para os dados, um interruptor para a imagem em espelho e as caixas da secção `[barcode]` (a largura da barra mais estreita, a altura, a distância entre as linhas, a potência, a velocidade, as passagens e os limites; um ficheiro de definições preenche-as); o código calcula-se com a mesma função de `phoenix-laser barcode` a partir da origem da máquina, e são recusados dados que não sejam texto imprimível ou sejam demasiado longos, um código acima de um limite, uma potência ou uma velocidade acima do limite da máquina ou um trabalho fora da mesa, com a caixa ou o motivo na linha de estado. Escreve-se como `barcode.nc` com o cabeçalho habitual.
* **A moldura na janela** (`src/window.rs`, `[frame]` no ficheiro de definições): um modo que enquadra a imagem ou o desenho aberto com o laser desligado; a margem, as voltas e a velocidade são caixas. Uma potência piloto acima de 0 liga o laser, por isso a janela pergunta e nada é calculado nem escrito até a pessoa responder que sim, de cada vez (mudar uma caixa, mudar de modo ou escrever o ficheiro retira a resposta), sob o teto de `max_pilot_percent`, o limite de potência e a mesa.
* **A pré-visualização na janela** (`src/window.rs`): todo o G-code que a janela calcula é contado pela mesma pré-visualização de `phoenix-laser preview` (movimentos, comprimento queimado e percorrido, um tempo que conta as esquinas com a aceleração da caixa e o que parece perigoso, como o trabalho fora da mesa do ficheiro de definições), e um programa de qualquer origem (`.nc`, `.ngc`, `.gcode`, `.gc`, `.tap`) pode ser aberto ou largado na janela para ver o seu trajeto e as suas notas. É lido como texto, nunca executado e nunca escrito de novo. A potência do laser pode ser escrita numa caixa (o ficheiro de definições e um perfil de máquina preenchem-na a partir de `laser_watts`; 0 significa que não é conhecida): a pré-visualização diz então também a energia que o programa queima e a sua potência média, como faz `phoenix-laser preview --laser-watts`, e um valor que não seja um número de 0 a 1000 é recusado.
* **O contorno na janela** (`src/window.rs`, `[trace]` no ficheiro de definições): um modo que segue o contorno da imagem aberta com a mesma função de `phoenix-laser trace` e o corta, ou preenche as suas formas com linhas, como uma camada chamada `trace`; as caixas são o limiar, a simplificação e a peça mínima, com um interruptor para uma tinta clara; a potência e a velocidade são as das caixas do laser e obedecem aos limites, e os limites da imagem (`max_pixels`, `max_contours`, `max_segments`) são os do ficheiro de definições, de modo que uma imagem demasiado detalhada é um erro e não um programa lento.
* **O perfil de máquina na janela** (`src/window.rs`, `[profile NAME]` no ficheiro de definições): uma lista no painel lateral com as máquinas do ficheiro; a escolhida é a máquina, tal como `--profile` a faz, de modo que as caixas do valor de potência total e dos limites de potência e de velocidade tomam os seus valores e a mesa do perfil é a que a moldura e a pré-visualização verificam. O que um perfil omite vem de `[machine]`, um perfil que não está no ficheiro não altera nada, e os limites continuam a recusar uma potência ou uma velocidade acima deles.
* **A grelha a partir de um material na janela** (`src/window.rs`, `[calibrate]` no ficheiro de definições): o modo de calibração lista os materiais da biblioteca; com um escolhido, a grelha reparte-se em redor da sua potência e da sua velocidade com a mesma escada de `phoenix-laser calibrate --material` (caixas para os passos, a margem e a potência e a velocidade mínimas, mantida entre elas e os limites), e um interruptor queima os algarismos junto aos quadrados com a altura da sua caixa. Sem nenhum escolhido é a grelha das duas listas, como antes.
* **A verificação do programa na janela** (`src/window.rs`, `[verify]` no ficheiro de definições): um programa de G-code aberto para ser visto é também verificado por `phoenix-laser verify` com os limites da janela e a cama do ficheiro de definições, e o painel lateral diz se passa (com o início do seu SHA-256) ou o que está mal; uma caixa guarda o sobrecurso permitido fora da cama com o laser desligado (0 por omissão, até 100 mm), e um programa que queima fora da cama falha diga o que disser a caixa.
* **A verificação de um programa** (`src/verify.rs`): `phoenix-laser verify PROGRAMA.nc` lê um programa G-code (deste programa ou de outro) como texto, nunca o executa, e dá o SHA-256 do ficheiro exato, o que faz e um veredicto: só passa abaixo do limite de potência e de velocidade da máquina, dentro da mesa (o trabalho fora dela falha; `--overscan-mm` ou `overscan_mm` em `[verify]` deixa a cabeça passar da mesa essa distância com o laser desligado, como o excesso de varrimento de outro programa, e queimar fora da mesa falha sempre), com o laser desligado em cada movimento rápido e avanço em cada movimento que queima. `--json` dá o relatório para outro programa guardar junto de uma autorização ligada a esse hash, com o perfil de máquina, o SHA-256 do ficheiro de definições de onde vêm os limites e os nomes das verificações feitas, e o código de saída é 1 quando falha. `report` escreve essa verificação como algo para ler antes de um trabalho, na língua de `--lang` (inglês, espanhol, alemão, francês, italiano, japonês, chinês ou português de Portugal): o SHA-256, os movimentos e o tempo, a potência e a velocidade máximas face aos limites, a energia, se o material estava verificado, as zonas proibidas e os bloqueios, e um veredicto.
* **Uma verificação ligada ao programa que foi autorizado** (`src/verify.rs`): `phoenix-laser verify PROGRAMA.nc --expect-sha256 HASH` faz falhar o programa quando o seu SHA-256 não são os 64 algarismos hexadecimais dados, e diz qual tem, de modo que um programa alterado depois de visto não é executado por engano; a falha está também no relatório JSON.
* **Zonas proibidas, bloqueios, o relatório e a lista de definições** (`src/guard.rs`, `src/report.rs`, `src/contract.rs`): os retângulos da cama que um programa tem de evitar (`[zone NOME]`) e o estado da paragem de emergência, da cobertura e da assistência de ar tal como o diz o ficheiro de definições (`[interlocks]`) fazem falhar a verificação e impedem o envio de um programa, de um trabalho vetorial e de um disparo de teste; o que vai para o servidor fica limitado a 40 % da potência, salvo se `[send]` indicar outro limite; `phoenix-laser report` escreve a verificação para ler antes de um trabalho, em oito línguas; `phoenix-laser settings` lista cada definição para que outro programa a possa ler. Nenhum sensor nem paragem de emergência real foi experimentado.
* **Envio para o servidor EH3D** (`src/server.rs`, `[server]` no ficheiro de definições): `phoenix-laser server send PROGRAMA.nc` envia um programa G-code como trabalho do plugin `laser` do EH3D-SERVER, com o cliente partilhado da suite (`eh3d-serverclient` do EH3D-COMMON: as chaves `server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries` e `server_mdns`, tentativas repetidas com pausas crescentes, um pedido que cria alguma coisa nunca é enviado duas vezes e as recusas do servidor são ditas pelo seu código na língua do sistema). Antes de sair, o programa é verificado contra os limites da máquina das definições (o limite de potência e de velocidade, a mesa) e um que falhe não é enviado; `--follow` fica até o trabalho terminar; `status`, `job`, `pause` e `resume` perguntam pela máquina e pelo trabalho e seguram-no ou deixam-no continuar; `stop` é a PARAGEM DE EMERGÊNCIA da máquina laser: o trabalho termina logo e a máquina fica bloqueada, de modo que não aceita nenhum programa, nem lhe é enviado nenhum, até um administrador executar `release`. O token nunca é impresso (pode estar na variável `PHOENIX_LASER_SERVER_TOKEN`) e viaja em claro por HTTP simples. Os comandos locais não são tocados. Testado contra um substituto do plugin laser, em memória e num socket deste computador; o plugin do servidor é ele próprio um simulador, e nada disto correu com um laser verdadeiro. `phoenix-laser server vector DRAWING.svg|dxf` envia um desenho como trabalho vetorial (os traçados e uma operação por camada, verificados antes contra os limites das definições) e `phoenix-laser server test-fire --power-percent P --confirm-fire` dispara o laser por um instante para encontrar o foco (o servidor recusa-o enquanto um trabalho corre, enquanto uma paragem de emergência retém a máquina, com o recinto aberto ou a assistência de ar desligada, e acima do seu próprio limite).
* **Envio para o servidor a partir da janela** (`src/window.rs`, `src/linkpane.rs`, `[server]` no ficheiro de definições): a secção do servidor da janela mostra o painel da suite (o endereço, o token, a máquina, os limites da ligação, o estado da ligação e um início de sessão com nome e palavra-passe), e o seu botão envia o programa calculado como trabalho da máquina laser, mostra o andamento e permite segurá-lo, deixá-lo continuar ou pará-lo com um botão de PARAGEM DE EMERGÊNCIA (o trabalho termina logo e a máquina fica bloqueada até um administrador a libertar); uma moldura que acende o laser e não foi confirmada e um programa que não passa a verificação dos limites não são enviados, e um programa aberto só para o ver é enviado tal como está apenas depois de passar a verificação e de a pessoa assinalar a caixa que o diz. Fechar a janela deixa o trabalho onde está no servidor.
* **Projetos LBRN2** (`src/lbrn2.rs`, `docs/lbrn2.md`, `examples/lbrn2_survey.rs`): `cut`, `vector`, `layers` e `convert` (e a janela) também leem projetos `.lbrn2`. Os retângulos, os caminhos (troços retos e curvas de Bézier), os grupos com as suas transformações e o contorno de um texto passam a ser um desenho em milímetros; cada camada do projeto passa a ser uma linha da tabela com a sua potência, velocidade (o projeto tem mm/s, a tabela mm/min), passagens, distância de preenchimento e ordem como sugestões sobre as quais continuam a valer os limites da máquina, e uma camada que não é enviada ao laser é ignorada. As imagens dentro de um projeto (um PNG ou um PGM em base64) são descodificadas dentro de limites e gravadas por pontos com `phoenix-laser engrave PROJETO.lbrn2 OUT.nc` (a potência e a velocidade da camada da imagem usam-se quando a linha de comandos não dá nenhuma, e um valor acima do limite é recusado, nunca baixado); também se leem uma elipse e um polígono (escritos apenas a partir dos nomes dos seus atributos, um protótipo), as pontes de uma camada e o contador do texto variável, e uma imagem acima de um limite ou noutro formato, e as formas de um tipo que não apareceu nos exemplos reais, ficam de fora e são contadas nas notas com um código estável. O formato é lido a partir do comportamento de doze projetos reais, sem especificação, e o eixo y é tomado como o de um SVG, salvo se `lbrn2_flip_y = off` disser outra coisa: o sentido nunca foi confirmado com uma peça real, e cada leitura di-lo nas suas notas. `phoenix-laser text OUT.nc --text 'N{counter:4} {name}' --csv FICHEIRO.csv` (ou `--counter INÍCIO,PASSO,QUANTOS`, ou `--counter-project PROJETO.lbrn2`) escreve um programa por cada linha de um ficheiro CSV, todos verificados antes de escrever o primeiro, e `--preview` mostra o texto de cada linha sem escrever nada. `cargo run --release --example lbrn2_survey -- PASTA` lê todos os ficheiros `.lbrn2` de uma pasta e diz quais são lidos, quais não e porquê, e que elementos e atributos os ficheiros têm que o leitor não usa.
* **Ficheiros de teste, o formato antigo de projetos e as predefinições em texto** (`src/testfiles.rs`, `src/presets.rs`, `src/dxf.rs`, `src/lbrn2.rs`): o leitor de DXF desenha o contorno de um `HATCH` e de um `SOLID`; os projetos `.lbrn` (o formato antigo, com os traçados escritos como elementos `V` e `P`) são lidos como os `.lbrn2` e os seus textos são desenhados com a fonte de traços; `--project-layers` dá às camadas de um projeto a sua potência, velocidade e passagens na linha de comandos; `calibrate` aceita intervalos (`--powers 20:80:20`); `phoenix-laser matrix` escreve a matriz de potência contra velocidade como um projeto com uma camada por cada par; `phoenix-laser ytest` escreve o projeto que mostra para onde aponta o eixo Y; um livro de predefinições guarda-se como texto e `phoenix-laser presets` lista-o. Não se queimou nada: a matriz e o teste do eixo Y não foram abertos noutro programa.
* **A janela de painéis** (`src/panels.rs`, `src/gui.rs`): a janela é a moldura de painéis da suite (`eh3d-dock`): treze painéis (anexos, vista, camadas, definições do trabalho, propriedades, biblioteca, materiais, enquadramento, máquina, simulação e tempo, trabalho, relatório e consola) que se podem retirar, repor, mover, agrupar em separadores, fazer flutuar, recolher e maximizar; quatro espaços de trabalho por tarefas, não por níveis de experiência (Preparar, Executar, Rever e Compacto); uma barra de menus com Ficheiro, Editar, Ver, Ferramentas, Janela e Ajuda; e, fora de qualquer painel para que nenhum espaço de trabalho os possa esconder, a paragem do trabalho e o estado dos bloqueios. A disposição guarda-se no ficheiro de definições como linhas que começam por `dock.`, que o leitor das definições deixa em paz. Os painéis, os espaços de trabalho e os menus são dados com testes; a janela não foi vista por uma pessoa num ecrã.
* **Pontos de partida da base comum de materiais** (`src/commonmat.rs`, funcionalidade `common-machining`): um material, um tipo de laser (díodo ou CO2), cortar ou gravar, a potência do laser em watts e a espessura dão a potência, a velocidade e as passagens que a base comum tem, tal como as tem, com a nota sempre à vista de que ninguém as experimentou numa máquina, e os motivos quando o material não deve encontrar um laser (então não há resposta). Fora do intervalo que a base conhece também não há resposta, nunca um palpite. A funcionalidade está ativa por omissão (a compilação está fixada numa versão do EH3D-COMMON que tem o módulo `machining`); `--no-default-features` deixa-a de fora.
* **Subcamadas** (`src/sublayers.rs`, `[sublayer CAMADA/ETIQUETA]` no ficheiro de definições): uma camada pode ser trabalhada mais do que uma vez, por exemplo um preenchimento e depois um contorno, cada passagem com o seu próprio modo, potência, velocidade, passagens, passo de foco e trama. Uma subcamada parte das definições da sua camada e é retida pelos limites da máquina tal como uma camada (um erro nomeia a camada e a etiqueta, e nada é baixado em silêncio); o trabalho vai primeiro a gravar e no fim a cortar, cada tipo pelo seu `order`, e `layers` lista as passagens com o seu tempo. O servidor recebe só as camadas, e di-lo.
* **Onde vai o trabalho, a sua moldura e o seu tempo** (`src/job.rs`, `[job]` no ficheiro de definições): o trabalho é colocado por um dos nove pontos da sua caixa (`anchor`), ou deixado onde foi desenhado com `align = page`; a moldura do trabalho é a sua caixa com a margem de `[frame]`, traçada com o laser desligado e recusada se saísse da cama; o tempo é um intervalo, desde cada movimento ao seu próprio avanço sem travagem até ao tempo da pré-visualização com travagem em cada esquina a `accel_mm_s2`, e as passagens estão em ambos.
* **Predefinições e ajuste em direto** (`src/presets.rs`, `src/live.rs`, `[live]` no ficheiro de definições): uma predefinição de um material e de uma espessura é atribuída a uma camada (uma cópia) ou ligada a ela (a camada segue a predefinição; uma alteração diz que camadas atingiu e é recusada por inteiro se uma não a puder aceitar). Os valores da base comum são usados tal como estão, marcados como não testados numa máquina, e os segundos valores de outras fontes não são usados a não ser que sejam pedidos. O ajuste em direto de potência e velocidade é um cálculo retido por quatro limites (o seu próprio intervalo, que por omissão nunca aumenta um valor, a máquina, o servidor e o piloto) e diz que limite reteve um valor; nada é enviado a uma máquina.
* **O relatório na janela** (`src/window.rs`): o relatório de `phoenix-laser report` pode ser feito a partir do programa que a janela calculou, no idioma da janela, e é descartado quando muda qualquer coisa.
* **Pontilhado ordenado** (`src/dither.rs`): `--dither bayer` (também na janela) decide cada ponto pelo seu lugar e pelo seu cinzento com uma matriz de Bayer de 8 x 8, por isso o padrão é regular e o mesmo cinzento queima sempre os mesmos pontos; os testes verificam que cada nível da matriz aparece uma vez e que a escuridão de um cinzento se mantém.
* **Uma verificação que lê como o controlador** (`src/preview.rs`, `src/verify.rs`): os espaços dentro de um número não contam; `M3` ligado enquanto a cabeça espera, está em pausa ou o programa termina, um comando `$` dentro do programa, uma parte que não se consegue ler, `G93`, os códigos M que outros controladores usam para um laser, os caracteres que o GRBL executa de imediato e um programa demasiado longo para ser lido por inteiro fazem falhar o `verify`, e nenhuma outra nota consegue tirar um perigo do relatório.
* **Definições do controlador e alarmes** (`src/grblset.rs`, `src/grblcodes.rs`): `phoenix-laser controller` compara a resposta guardada a `$$` com a máquina (modo laser, potência máxima `$30`, potência mínima, polegadas, curso), `verify --controller` junta-o ao veredicto e `phoenix-laser explain` diz o que significa um alarme ou um erro, nas oito línguas; nada é enviado para uma máquina e não foram experimentadas listagens nem alarmes de um controlador real.
* **Testes de espaçamento das linhas e de largura do feixe** (`src/calibrate.rs`): `phoenix-laser interval` preenche uma fila de quadrados com vários espaçamentos e `phoenix-laser kerf` corta um retângulo em tiras cuja folga dá a largura do feixe; só são escritos se passarem a verificação, e nada foi queimado.
* **Mais tramados e linhas cortadas duas vezes** (`src/dither.rs`, `src/cut.rs`): Jarvis, Stucki, Sierra e Burkes (também na janela), e um aviso quando há troços de uma camada de corte desenhados duas vezes.
* **A curva de tons** (`src/tone.rs`): antes de fazer os pontos, os cinzentos podem ser esticados a toda a gama (`--auto-level`), receber mais ou menos contraste e brilho, ser curvados com um gama ou invertidos, com uma única tabela de 256 entradas; com os valores por omissão a imagem não muda de todo.
* **Um G-code verificado:** o programa de um desenho é verificado linha a linha antes de ser entregue: nenhum laser ligado durante um movimento rápido ou sem velocidade, nenhum `S` acima da potência máxima, nenhum movimento fora da mesa, `M5` no fim; o ficheiro nunca é escrito por cima do desenho nem das definições de que provém.
* **Projetos da suite** (`src/eh3dfile.rs`, `docs/eh3d-project.md`): `phoenix-laser project` guarda o trabalho no ramo `laser/` de um projeto `.eh3d`, um só ficheiro com um ramo por programa: `save` coloca lá o desenho, as definições, a biblioteca de materiais e os programas; `import` conserva byte a byte um desenho de outro programa (LBRN2, LBRN, SVG ou DXF) com um relatório do que foi lido, do que não foi e do que se perde na exportação; `take` copia um contorno ou uma malha de outro ramo, como `cad/`, com o hash e o formato da origem, e `list` e `open` avisam quando está desatualizado; `open` e `extract` escrevem ficheiros sem substituir nenhum. Só `laser/` é escrito: os outros ramos, conhecidos ou não, conservam os seus bytes. Só foi experimentado com projetos inventados.
* **O projeto na janela** (`src/suitewin.rs`, `eh3d_project::suiteview`, `docs/eh3d-project.md`): Ficheiro > Projeto da suite (.eh3d) abre um projeto, guarda-o e guarda-o com outro nome conservando todos os ramos, importa um ficheiro guardando o original, obtém um ficheiro de outro ramo, extrai um ramo e lista os ramos. O que está desatualizado ou assinado fica à vista; antes de escrever sobre um projeto que não foi aberto ali, um ramo que mudou desde que foi lido ou uma assinatura, a janela pergunta (Esc responde que não e Enter não escreve). Abrir um projeto nunca alivia os limites, os encravamentos nem o servidor das definições próprias, e um ficheiro de definições com o token do servidor não é colocado num projeto. A lógica é testada sem ecrã; nenhuma pessoa experimentou as entradas num ecrã. Um projeto que outro programa guardou desde que foi lido aqui não é escrito por cima, nem quando foi guardado enquanto uma pergunta esperava: a janela di-lo e propõe lê-lo de novo ou guardar com outro nome.
* **Anexos do projeto** (painel «Anexos», o crate `eh3d-attachpanel` do EH3D-COMMON): está em Ver e em Ficheiro > Projeto da suite, sobre o projeto da janela. As folhas de dados, os manuais e qualquer outro ficheiro guardado no projeto `.eh3d` são listados, anexados (um ficheiro ou apenas uma ligação), ligados a um componente, extraídos e removidos depois de uma pergunta, e mostrados ao lado da lista (páginas PDF, imagens, texto e o texto de documentos de escritório). Os programas, scripts, ficheiros com macros, ficheiros comprimidos, HTML, SVG e tipos desconhecidos perguntam antes de sair do projeto; um programa ou um script nunca é aberto, nada é transferido e um ficheiro de definições com o token do servidor não é anexado. O pacote inclui a biblioteca de PDF (PDFium) com as suas licenças. Nenhuma pessoa o experimentou num ecrã. Pode largar um ficheiro sobre o painel, e os seus limites são lidos no ficheiro de definições (`attachments.max_mb`, `viewer.max_*_mb`).

## 🧩 Papel no EH3D

EH3D-PHOENIX-LASER é um programa da EH3D Fabrication Suite. Funciona como biblioteca e linha de comandos e, compilado com `--features gui`, como janela com o aspeto que os outros programas da suíte partilham. Lê e escreve ficheiros simples, por isso pode ser usado sozinho ou entre os outros programas.

## 🏛️ Arquitetura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitetura de EH3D-PHOENIX-LASER: quem o usa, o que contém e de que depende" width="100%">
</p>


Como as partes deste repositório se encaixam: quem o usa, o que contém e de que depende. A imagem é desenhada a partir de `docs/architecture.json` e verificada com o código.

## 🖼️ Capturas de ecrã

Estas imagens são tiradas pelo próprio programa com `--shot` (ver Início rápido).

<p align="center">
  <img src="images/screens/window-vector.png" alt="Corte de um desenho: o percurso do laser sobre a placa dos exemplos, com a tabela de camadas por baixo" width="100%">
</p>

<p align="center">
  <img src="images/screens/window-engrave.png" alt="Gravação de uma imagem: os pontos do gradiente dos exemplos coloridos pela sua potência" width="100%">
</p>

## ✨ Funcionalidades

* **Biblioteca e linha de comandos:** cada comando de `phoenix-laser` é uma função da biblioteca, por isso outro programa da suíte pode chamá-la sem lançar um processo.
* **Linha de comandos estrita:** uma opção mal escrita (`--powr`) ou uma palavra a mais é um erro que a nomeia, não algo que se ignora; caso contrário o programa faria a sua saída com os valores por omissão.
* **Testes com entradas danificadas:** os testes também dão ficheiros vazios, lixo, números enormes e ficheiros cortados, e nenhum pode entrar em pânico.
* **Sem adivinhar:** um valor sem sentido é um erro que diz qual e porquê, nunca um resultado que apenas parece certo.
* **Sem código unsafe:** `#![forbid(unsafe_code)]` na biblioteca e no programa.

## 🔒 Segurança e proteção

| Ameaça | Defesa |
|---|---|
| Um ficheiro danificado, cortado ou vazio | sem pânico: os testes cortam uma entrada válida em cada posição e dão lixo |
| Um número sem sentido (negativo, não finito, absurdamente grande) | recusado com um erro que nomeia o valor; nada é arredondado para algo plausível |
| Uso inseguro de memória | Rust seguro e simples com `forbid(unsafe_code)` |
| Um desenho hostil (scripts, entidades declaradas, aninhamento profundo, milhões de pontos, números enormes) | os scripts, as imagens e as ligações nunca são executados nem descarregados; uma entidade declarada é um erro; o tamanho, a profundidade, os elementos, as formas, os pontos e os troços de uma curva são limitados; uma forma com um ponto fora do intervalo fica de fora |
| Um programa que poderia ligar o laser sem velocidade ou acima da potência máxima | cada programa é verificado linha a linha antes de ser entregue e o que falha não é escrito |
| Uma potência superior à que a máquina ou o material devem receber | o limite de potência (`max_power_percent`, `--max-power`) recusa um programa que o ultrapasse com um erro que nomeia a camada ou a opção; nunca é baixada sem aviso |
| Uma biblioteca de materiais estragada ou enorme | o ficheiro tem limite de tamanho e de número de materiais; um valor fora do intervalo, uma chave desconhecida ou um nome repetido é um erro que nomeia a linha; a saída nunca é escrita por cima dela |
| Um programa G-code de fora que se lê antes de o executar | a pré-visualização só lê texto, com limite de tamanho, nunca executa nem descarrega nada, ignora números absurdos e avisa de laser ligado num movimento rápido, sem avanço, potência acima da plena, trabalho fora da mesa e sem fim |
| O laser a ligar só para posicionar o trabalho | a moldura é percorrida com o laser desligado (sem potência, o fuso nunca ligado); uma potência piloto exige `--confirm-laser-on` em cada execução, mesmo que o ficheiro de definições a peça, fica no teto ou abaixo (5 % se `max_pilot_percent` não indicar outro), sob o limite de potência e é desligada antes de o programa acabar |
| Uma cabeça enviada no sentido errado em Z por um teste de foco | o passo de altura não tem valor por omissão e o teste é recusado até a pessoa o dar; o programa avisa de que move Z, o curso é limitado (50 mm no total), a cabeça volta à primeira altura e a potência obedece ao limite de potência |
| Um texto demasiado comprido, alto ou forte, ou que gravaria fora da mesa | só se aceitam os caracteres do tipo de letra (uma letra que não tem é recusada, nunca saltada), o número de caracteres, a altura, a potência e a velocidade mantêm-se entre limites que as definições podem apertar, o trabalho fora da mesa é recusado em vez de cortado e o texto nunca é escrito num comentário do programa |
| Dados que não cabem no código QR, um símbolo demasiado grande, denso ou forte, ou que gravaria fora da mesa | a versão, o número de bytes, o tamanho, o número de movimentos de gravação, a potência e a velocidade mantêm-se entre limites que as definições podem apertar, e o que os ultrapassa é recusado em vez de cortado ou reduzido; os dados nunca são escritos num comentário do programa |
| O ficheiro de saída escolhido por cima do desenho ou das definições de que provém | recusado, na janela e nos comandos |
| Um programa enviado para um servidor, um token que poderia ser visto, ou uma máquina que foi parada e voltou a arrancar | o programa é verificado contra os limites da máquina antes de sair e um que falhe não é enviado; o token nunca é impresso (nem numa mensagem nem num `Debug`), vem do ficheiro de definições ou do ambiente e viaja em claro por HTTP simples, por isso é para uma rede de confiança ou um túnel (o programa diz-o quando o endereço é público); a paragem é a de emergência, que bloqueia a máquina até um administrador a libertar, e a uma máquina bloqueada não se envia nenhum programa |

**O que não é defendido:** um resultado produzido sem erros não é um resultado verificado. Nada aqui conhece os limites de uma máquina e o programa não teve revisão independente. Comunique uma vulnerabilidade em privado (ver [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

A biblioteca é `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`); a linha de comandos é `phoenix-laser`. Os comandos são estes; cada um imprime o resultado na saída padrão e comunica um erro na saída de erros com um código de saída diferente de zero. Não há acesso à rede.

```text
phoenix-laser engrave IMAGE OUT.nc --width-mm 50 [--dot-mm 0.1] [--feed 3000] [--power 50] [--smax 1000]
              [--dither fs|atkinson|threshold|bayer|jarvis|stucki|sierra|burkes] [--threshold 128] [--overscan MM|auto] [--accel 1000] [--origin X,Y] [--dry-run]
              [--gamma 1] [--contrast 0] [--brightness 0] [--invert] [--auto-level] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
              [--material NAME --materials FILE]
phoenix-laser calibrate OUT.nc --powers 20,40,60,80|20:80:20 --feeds 1000,2000,3000|1000:3000:1000 [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser calibrate OUT.nc --material NAME --materials FILE [--steps 5] [--spread 40] [--labels|--no-labels] [--label-mm 3] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser matrix OUT.lbrn2 --powers 20:80:20 --feeds 600:2400:600 [--square-mm 8] [--gap-mm 3] [--outline]
phoenix-laser ytest OUT.lbrn2
phoenix-laser presets BOOK.cfg [--lang CODE]
phoenix-laser text OUT.nc --text TEXT [--height-mm 10] [--spacing 25] [--line-spacing 50] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--align left|center|right] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser qr OUT.nc --text DATA [--ecc L|M|Q|H] [--module-mm 1] [--line-mm 0.1] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--invert] [--border-modules 0] [--overscan-mm 0] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--print] [--config FILE] [--profile NAME]
phoenix-laser barcode OUT.nc --text DATA [--module-mm 0.4] [--height-mm 10] [--line-mm 0.1] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser marks OUT.nc --size W,H [--origin X,Y] [--arm-mm 5] [--margin-mm 0] [--power 10] [--feed 1000] [--passes 1] [--centre-cross|--no-centre-cross] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser info IMAGE
phoenix-laser cut DRAWING OUT.nc [--config FILE] [--project-layers] [--power P] [--speed MM/MIN] [--passes N] [--focus-step MM] [--kerf MM] [--kerf-side none|auto|inside|outside]
              [--tabs N] [--tab-width MM] [--smax N] [--overscan MM] [--origin X,Y] [--bed X,Y] [--max-power P] [--max-speed MM/MIN] [--profile NAME] [--material NAME --materials FILE] [--dpi N] [--tolerance MM] [--dry-run]
phoenix-laser vector DRAWING OUT.nc [the options of cut] [--angle DEG] [--spacing MM] [--cross|--no-cross]
phoenix-laser layers DRAWING [--config FILE] [--project-layers] [--dpi N] [--accel MM/S2]
phoenix-laser convert DRAWING OUT.svg|dxf [--config FILE] [--dpi N] [--tolerance MM]
phoenix-laser trace IMAGE OUT.nc --width-mm W [--threshold 128] [--invert] [--simplify MM] [--min-area MM2] [--engrave] [the options of cut]
phoenix-laser materials FILE
phoenix-laser focus OUT.nc --z-step MM [--z-start MM] [--steps 9] [--length 10] [--spacing 2] [--feed 1000] [--power 30 | --powers 10,20,30] [--smax 1000] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser frame OUT.nc --size W,H | --gcode PROGRAM.nc [--origin X,Y] [--margin MM] [--feed 3000] [--laps 1] [--bed X,Y] [--smax N]
              [--pilot P --confirm-laser-on] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser verify PROGRAM.nc [--config FILE] [--profile NAME] [--smax N] [--max-power P] [--max-speed MM/MIN] [--bed X,Y] [--overscan-mm MM] [--accel 1000] [--expect-sha256 HASH] [--controller SETTINGS.txt] [--json]
phoenix-laser report PROGRAM.nc [--config FILE] [--profile NAME] [--lang CODE] [--material NAME --materials FILE]
phoenix-laser controller SETTINGS.txt [--config FILE] [--profile NAME] [--smax N] [--bed X,Y] [--lang CODE]
phoenix-laser explain ALARM:n|error:n [--lang CODE]
phoenix-laser interval OUT.nc --spacings 0.08:0.2:0.02 --power P --feed MM/MIN [--square-mm 8] [--gap-mm 3] [--label-mm 2|--no-labels] [--smax N] [--max-power P] [--max-speed MM/MIN] [--bed X,Y] [--config FILE] [--profile NAME]
phoenix-laser kerf OUT.nc --power P --feed MM/MIN [--strips 10] [--strip-mm 5] [--height-mm 20] [--passes 1] [--smax N] [--max-power P] [--max-speed MM/MIN] [--bed X,Y] [--config FILE] [--profile NAME]
phoenix-laser kerf --gap MM [--strips 10]
phoenix-laser settings [--json]
phoenix-laser project list FILE.eh3d [--json]
phoenix-laser project save FILE.eh3d [--drawing D] [--config FILE] [--materials FILE] [--program P.nc] [--name NAME] [--id ID]
phoenix-laser project import FILE.eh3d DRAWING [--name NAME] [--id ID]
phoenix-laser project open FILE.eh3d [--into DIR] | take FILE.eh3d BRANCH NAME | extract FILE.eh3d BRANCH DIR
phoenix-laser preview PROGRAM.nc [OUT.svg] [--accel 1000] [--rapid 6000] [--smax N] [--laser-watts W] [--bed X,Y] [--width-px 900] [--config FILE] [--profile NAME]
phoenix-laser align PICTURE.pgm|png --size W,H --px-per-mm S [--origin X,Y] [--arm-mm 5] [--margin-mm 0] [--centre-cross|--no-centre-cross] [--threshold 128] [--max-shift-mm 20] [--max-angle-deg 5]
              [--config FILE] [--profile NAME] | --simulate OUT.pgm --size W,H --px-per-mm S [--shift DX,DY] [--turn DEG] [--picture-px W,H]
phoenix-laser server status|release [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME]
phoenix-laser server send PROGRAM.nc [--name TEXT] [--follow] | job|pause|resume|stop JOB [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME]
phoenix-laser server vector DRAWING.svg|dxf [--name TEXT] [--follow] [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME] [--material NAME] [--power P] [--speed MM/MIN] [--passes N]
phoenix-laser server test-fire --power-percent P [--seconds S] --confirm-fire [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME]
phoenix-laser [gui] [FILE]                       the window (a build with --features gui); with no arguments it opens
```

`DRAWING` é um ficheiro SVG ou DXF. `cut` segue os contornos (os de dentro primeiro); `vector` preenche as formas fechadas com linhas e segue as abertas; ambos tomam as definições de cada camada ou cor de `--config FILE` (ver [docs/vector-settings.md](docs/vector-settings.md)) e uma opção da linha de comandos altera-as em todas as camadas. `layers` lista as camadas de um desenho com o que se faria com cada uma. `convert` escreve um desenho de novo como SVG ou DXF simples (conforme a extensão da saída, nunca por cima do desenho lido), para que outro programa possa abrir o que este percebeu.

## ⚙️ Configuração

A gravação de uma imagem define-se com argumentos da linha de comandos (ver Interfaces) e cada valor por omissão está escrito na ajuda do comando (`phoenix-laser help`). O corte e a gravação de um desenho definem-se com um ficheiro de definições (`--config FILE`, ou «Carregar definições» na janela): os limites dos leitores, como se lê um ficheiro, a ordem dos cortes, a máquina e, para cada camada ou cor, o modo (cortar, gravar ou ignorar), a potência, a velocidade, as passagens, o avanço do foco, a largura do feixe e o seu lado, as pontes e a hachura. Cada chave, o seu valor por omissão e o seu intervalo estão em [docs/vector-settings.md](docs/vector-settings.md) e há um exemplo em `examples/plate.cfg`; uma chave desconhecida ou um valor fora do intervalo é um erro que nomeia a sua linha. Os limites fixos do leitor de imagens estão no código como constantes com nome e são cobertos por testes. `phoenix-laser settings` lista cada definição (secção, chave, tipo de valor, valor por omissão e se é uma definição de segurança) como texto, ou como JSON com `--json`, para que outro programa a possa ler.

## 🚀 Início rápido

Precisa de uma cadeia de ferramentas Rust estável (1.88 ou mais recente). O repositório EH3D-COMMON tem de estar clonado ao lado deste: o leitor de imagens é tomado dele por caminho.

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/plate.svg   # a picture of the window
```

Para tirar uma captura da janela sem a abrir à mão, execute-o com `--shot FICHEIRO.png` (também `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--window NAME`, `--lang CODE`, `--open FILE`, `--replace`): abre-se com esse tamanho, guarda o PNG, não guarda definições e fecha-se sozinho (código de saída 0 se o ficheiro foi escrito, 1 se não, 2 se os argumentos estiverem errados). `--window NOME` mostra uma janela do programa: `engrave`, `calibrate`, `cut`, `focus`, `frame`, `preview`, `trace`, `text`, `qr`, `marks`, `barcode` (esse modo fica escolhido) ou `server` (a secção do servidor fica aberta), para que as capturas do README possam mostrar cada modo. `--window` também aceita `prepare`, `run`, `review` e `compact`, os quatro espaços de trabalho dos painéis.

## 🧪 Testes

O conjunto tem 1094 testes (`cargo test`, executados em Windows e Linux no CI). Verificam os casos normais, os casos-limite e as entradas danificadas: ficheiros vazios, lixo e ficheiros cortados nunca provocam pânico. O CI executa ainda `cargo fmt --check`, `cargo clippy` com avisos como erros e `python -I tools/ci_validate.py`.

## 🏗️ Compilação e publicação

A versão está em `Cargo.toml` e em `eh3d.project.json`; cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilam o programa, `test.bat` / `test.sh` executam os testes e `run.bat` / `run.sh` executam-no com um exemplo. As versões são publicadas no GitHub.

**Transferências:** Com cada versão deste repositório são publicados programas já feitos para cada sistema: Windows (64 e 32 bits), Linux (64 bits) e macOS (Apple silicon e Intel). Cada pacote, `phoenix-laser-<versão>-<sistema>.zip` ou `.tar.gz`, leva o programa, a licença, este README, o registo de alterações, o ícone e os ficheiros de exemplo, e vem com o seu ficheiro `.sha256`; a versão também lista todos os ficheiros em `SHA256SUMS` e `build-manifest.json`. As versões estão listadas na página Releases do repositório.

**Assinados e verificáveis:** os pacotes de uma versão são assinados com a chave de versões da EH3D, e o ficheiro `.sig` de cada pacote é publicado ao lado (uma assinatura OpenSSH, espaço de nomes `eh3d-release`). Verifique uma transferência antes de a executar: compare o seu SHA-256 (`sha256sum -c phoenix-laser-<versão>-<sistema>.tar.gz.sha256`, ou `Get-FileHash` no Windows) e verifique a assinatura com `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <pacote>.sig < <pacote>`, onde `allowed_signers` contém a linha `eh3d-release <tipo de chave> <chave pública das versões da EH3D>`.

## 📂 Estrutura do repositório

<p align="center">
  <img src="images/repository-structure.svg" alt="Estrutura do repositório EH3D-PHOENIX-LASER" width="100%">
</p>

<details>
<summary>Versão em texto da árvore</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   a biblioteca: o que cada comando usa
│   ├── main.rs                                  a linha de comandos: engrave, calibrate, cut, vector, verify, report, convert, settings e mais
│   ├── calibrate.rs                             as grelhas de teste: potência contra avanço, escada de foco e grelha de um material
│   ├── cli_args.rs                              os argumentos da linha de comandos, o mesmo ficheiro em cada utilitário pequeno
│   ├── grblset.rs                               as definições guardadas de um controlador face à máquina
│   ├── grblcodes.rs                             o significado dos alarmes e erros do GRBL, em oito línguas
│   ├── testcli.rs                               os comandos interval, kerf, controller e explain
│   ├── dither.rs                                dos cinzentos aos pontos: limiar, Bayer e seis formas de repartir o erro
│   ├── materials.rs                             a biblioteca de materiais: potência, velocidade, passagens, tons
│   ├── commonmat.rs                             pontos de partida da base comum de materiais, tal como estão
│   ├── preview.rs                               a pré-visualização de um G-code: movimentos, tempo, perigos e um SVG do percurso
│   ├── tone.rs                                  a curva de tons de uma imagem: níveis, contraste, brilho, gama e inversão antes dos pontos
│   ├── trace.rs                                 os contornos de uma imagem: limiar, marching squares e anéis simplificados, com limites
│   ├── text.rs                                  uma linha de texto queimada com uma fonte de traços própria, dentro de limites
│   ├── qr.rs                                    um código QR próprio, versões 1 a 4 com correção de erros, queimado a preencher os módulos
│   ├── barcode.rs                               um código de barras Code 128 próprio, queimado a preencher as barras com linhas
│   ├── gcode_out.rs                             a única forma de escrever um número de G-code e o cabeçalho padrão
│   ├── raster.rs                                a gravação linha a linha: de pontos a movimentos, sobrecurso e potência
│   ├── safety.rs                                o limite de potência e as regras que mantêm o laser desligado ao mostrar o trabalho
│   ├── guard.rs                                 as zonas proibidas da cama e os bloqueios: o que um programa não pode alcançar
│   ├── server.rs                                o caminho para o servidor EH3D: verificar, enviar, seguir, paragem de emergência
│   ├── linkpane.rs                              o painel do servidor na janela: as caixas, a ligação e o início de sessão
│   ├── shot.rs                                  o modo de captura da janela (--shot): os seus argumentos, lidos e verificados sem ecrã
│   ├── vector.rs                                um desenho lido: formas em mm, camadas, limites e a geometria básica
│   ├── verify.rs                                a verificação de um programa: o seu SHA-256, o que faz e um veredicto face aos limites da máquina
│   ├── report.rs                                o relatório lido antes de um trabalho, nas oito línguas
│   ├── flatten.rs                               curvas em troços retos dentro de uma tolerância (Bézier, arcos)
│   ├── frame.rs                                 a moldura do trabalho (o retângulo percorrido com o laser desligado) e as marcas dos seus cantos
│   ├── register.rs                              onde está o trabalho: as marcas encontradas numa imagem e a câmara simulada
│   ├── svg.rs                                   o leitor de SVG: um subconjunto seguro, sem scripts, imagens nem entidades
│   ├── testfiles.rs                             a matriz de potência contra velocidade e o teste do eixo Y, como projetos
│   ├── dxf.rs                                   o leitor de DXF: linhas, polilinhas, arcos, círculos, elipses, splines e blocos, com unidades
│   ├── eh3dfile.rs                              o ramo laser de um projeto .eh3d: guardar, importar, tomar, abrir, extrair
│   ├── projcli.rs                               o comando project da linha de comandos
│   ├── suitewin.rs                              o menu Ficheiro da janela para um projeto .eh3d: abrir, guardar, importar, obter, extrair
│   ├── lbrn2.rs                                 o leitor de projetos LBRN2: camadas, formas, grupos e curvas
│   ├── lbrn2_pic.rs                             as imagens de um projeto LBRN2: base64, PNG, limites, rodadas como colocadas
│   ├── vartext.rs                               o texto variável: um contador e as colunas de um CSV, um texto por linha
│   ├── sublayers.rs                             passagens extra sobre uma camada: definições próprias, limites e ordem do trabalho
│   ├── job.rs                                   onde o trabalho começa, a moldura que o mostra e o tempo como intervalo
│   ├── live.rs                                  o ajuste em direto de potência e velocidade, com quatro limites (nenhuma máquina é tocada)
│   ├── presets.rs                               predefinições de material: atribuir (copiar) ou ligar (seguir), com aviso dos valores não testados
│   ├── export.rs                                um desenho escrito de novo como SVG ou DXF simples
│   ├── energy.rs                                a energia de uma passagem por milímetro ou milímetro quadrado, a partir da potência ótica
│   ├── offset.rs                                deslocar um contorno de lado pela largura do feixe, verificado aresta a aresta
│   ├── order.rs                                 a ordem dos cortes: linhas unidas, o interior primeiro, pouco deslocamento
│   ├── cut.rs                                   do desenho ao G-code: corte, pontes, hachura, passagens e a verificação de segurança
│   ├── vconfig.rs                               o ficheiro de definições do trabalho vetorial: limites, máquina e cada camada
│   ├── contract.rs                              cada definição em lista: secção, chave, tipo, valor por omissão e se é de segurança
│   ├── browse.rs                                a lista de ficheiros da janela que abre um ficheiro
│   ├── gui.rs                                   a janela (egui, com a funcionalidade gui): desenha o estado e transmite o que a pessoa faz
│   ├── panels.rs                                os painéis, espaços de trabalho e menus da janela como dados
│   ├── lang.rs                                  as palavras da janela nos oito idiomas
│   ├── viewport.rs                              zoom e deslocamento da imagem, em números simples
│   └── window.rs                                o estado da janela sem ecrã: imagem ou desenho, definições, camadas, passagens e exportação
├── tests/
│   ├── fuzz.rs                                  testes com semente fixa: ficheiros danificados, cortes e números estranhos
│   ├── dry_run.rs                               testes de --dry-run: conta o mesmo e não escreve nada
│   ├── laser.rs                                 testes da biblioteca e da linha de comandos
│   ├── laser_tools.rs                           testes das ferramentas à volta da gravação, pela linha de comandos
│   ├── readme.rs                                testes que executam o que o README e os scripts run prometem
│   ├── strict.rs                                testes da linha de comandos: opções e palavras que não conhece
│   ├── vector.rs                                testes do corte a partir de desenhos: comandos, exemplos, ficheiros hostis e a verificação do G-code
│   ├── lbrn2.rs                                 testes do leitor LBRN2 com projetos escritos aqui, ficheiros hostis e fuzz
│   ├── lbrn2_more.rs                            testes das imagens LBRN2, do eixo y, elipse, polígono, pontes e fuzz
│   ├── faults.rs                                testes do que a verificação de um programa recusa
│   ├── grblset.rs                               testes das definições do controlador, alarmes e erros
│   ├── testcuts.rs                              testes dos programas de espaçamento e largura do feixe e do tramado
│   ├── lbrn_v1.rs                               testes do formato antigo de projetos, dos seus textos e das camadas de um projeto na linha de comandos
│   ├── dxf_hatch.rs                             testes dos preenchimentos e sólidos de um DXF, com fuzz de semente fixa
│   ├── vartext.rs                               testes do texto variável: CSV, contadores, lugares, séries e fuzz
│   ├── sublayers.rs                             testes das passagens extra: ordem, limites, ficheiros danificados com semente fixa
│   ├── joborigin.rs                             testes dos nove âncoras, da moldura com o laser desligado e do tempo como intervalo
│   ├── livelimits.rs                            testes do ajuste em direto: quatro limites, oito línguas, semente fixa
│   ├── presets.rs                               testes das predefinições: atribuir, ligar, alterações recusadas, alternativas e série aleatória
│   ├── presetbook.rs                            testes do livro de predefinições guardado como texto
│   ├── matrix.rs                                testes dos intervalos da grelha de teste e da sua matriz
│   ├── testfiles.rs                             testes dos projetos de matriz e de eixo Y que o programa escreve
│   ├── samples.rs                               os leitores sobre uma pasta de amostras, quando se indica uma
│   ├── guard.rs                                 testes das zonas, dos bloqueios, dos tetos de material, do relatório e da lista de definições
│   ├── server_cli.rs                            o comando server contra um substituto do plugin laser
│   ├── window_cli.rs                            testes de que uma compilação sem a funcionalidade gui continua a ser só linha de comandos
│   ├── dockwindow.rs                            testes do modelo da janela: camadas de um projeto, relatório, bloqueios, disposição e materiais
│   ├── eh3dfile.rs                              testes do ramo do projeto: ida e volta, outros ramos, cópias desatualizadas, ficheiros danificados
│   ├── suitewindow.rs                           testes do menu de projeto da janela, sem ecrã
│   ├── reqs.rs                                  testes do registo de requisitos
│   ├── more.rs                                  o G-code relido, os números e a grelha
│   ├── more2.rs                                 a gravação no limite de cada valor
│   └── rapid_power.rs                           nenhum movimento rápido com potência definida
├── examples/
│   ├── gradient.pgm                             um exemplo para experimentar o programa
│   ├── plate.svg                                uma placa com um furo e um painel para gravar (SVG)
│   ├── plate.dxf                                a mesma placa em DXF
│   ├── plate.cfg                                as definições das suas camadas: potência, velocidade, passagens, largura do feixe, pontes e hachura
│   ├── materials.cfg                            uma biblioteca de materiais: vidoeiro e ardósia, corte, gravação e fotografias
│   ├── safety.cfg                               zonas proibidas e bloqueios para experimentar com verify
│   ├── materials-starter.cfg                    uma biblioteca inicial de materiais, todos marcados como não verificados
│   └── lbrn2_survey.rs                          lê todos os ficheiros LBRN2 de uma pasta e diz o que se lê e o que não
├── docs/
│   ├── architecture.json                        dados da imagem de Arquitetura
│   ├── NOT_YET.md                               boas ideias fora desta versão, com o porquê e o quando
│   ├── REQUIREMENTS.md                          o registo de requisitos: estado, evidência e teste de cada um
│   ├── SCOPE.md                                 para que serve, o que não fará e o que nunca decide
│   ├── STATUS.md                                o que existe, o que falta e o que continua em aberto
│   ├── structure.json                           dados da imagem de Estrutura
│   ├── vector-settings.md                       o ficheiro de definições do trabalho vetorial, chave a chave
│   ├── materials.md                             a biblioteca de materiais, chave a chave
│   ├── eh3d-project.md                          o projeto .eh3d e o ramo laser
│   └── lbrn2.md                                 o que o leitor LBRN2 lê, o que deixa de fora e como foi medido
├── tools/                                       verificação da CI e do README e ferramentas de imagens
├── images/                                      banner, imagens do README e capturas de ecrã da janela
├── .github/                                     fluxos de CI e de publicação, modelos de issues e pull requests
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── Cargo.toml Cargo.lock                        o workspace e as suas dependências fixadas
├── rust-toolchain.toml                          a toolchain de Rust fixada
├── CHANGELOG.md                                 o que mudou em cada versão
├── AGENTS.md                                    as regras para quem altera o repositório
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

</details>

## 🗺️ Roteiro e limites conhecidos

**Limites conhecidos hoje**

* Não foi experimentado numa máquina nem com uma coleção de ficheiros reais e não teve revisão independente: nada foi queimado, as verificações leem o programa como texto e o servidor é um substituto.
* O leitor de projetos LBRN2 foi medido com doze projetos reais de um fabricante de lasers e com mais nada: as imagens dentro de um projeto, as formas de um tipo que não estava entre eles, o sentido do eixo y e o canto arredondado de um retângulo não estão confirmados.
* Um desenho é lido num subconjunto seguro (sem `use` de outro ficheiro, texto, gradientes nem estilos com seletores; sem hachuras, texto nem cotas num DXF); um contorno mais estreito do que o feixe não é cortado e o programa di-lo; o laser é o de um controlador GRBL com `M4`.
* O código QR e o código de barras são relidos por leitores do próprio programa; nenhum leitor ótico leu um símbolo queimado, e a janela não foi vista por uma pessoa num ecrã.
* As subcamadas, as ligações das predefinições e o ajuste em direto ainda não têm lugar na janela; as ligações das predefinições ficam em memória e não no ficheiro de definições, e nada do ajuste em direto é enviado a uma máquina.

**A seguir**

* Uma primeira prova num laser e num material reais, com uma pessoa na máquina e a paragem de emergência ao alcance.
* Mostrar a densidade de potência na placa antes de queimar (`layers` já escreve a energia de uma passagem quando `laser_watts` está definido) e manter constante a potência nos cantos de um corte.
* Ver a janela num ecrã e terminá-la.
* Experimentá-lo com desenhos reais e com os outros programas da suite.
* As ideias deixadas de fora de propósito estão em [Ainda não](docs/NOT_YET.md).

## 🌍 Idiomas

O README e os textos existem em inglês, espanhol, francês, italiano, alemão, chinês, japonês e português europeu.

## 🔗 Projetos relacionados

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): os crates partilhados, entre eles o leitor de imagens
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): os stencils de pasta de solda
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): a verificação do G-code antes de chegar a uma máquina
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): o armazém de ferramentas partilhado pelos programas CAM
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): o programa de controlo de máquina
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): o centro que encontra e arranca os programas da suíte

## 📚 Documentação

* [Registo de alterações](CHANGELOG.md)
* [Âmbito](docs/SCOPE.md): para que serve o programa, o que não fará e o que nunca decide.
* [Requisitos](docs/REQUIREMENTS.md): o registo das capacidades, o estado de cada uma e o teste que a mostra.
* [Estado](docs/STATUS.md): o que existe, o que falta e o que continua em aberto.
* [Ainda não](docs/NOT_YET.md): boas ideias que ficam fora desta versão.
* [Regras para alterar o repositório](AGENTS.md): as regras para quem altera o repositório.
* [Dados de arquitetura e estrutura](docs/architecture.json): os dados a partir dos quais as duas imagens são desenhadas.
* [Definições do trabalho vetorial](docs/vector-settings.md): cada chave do ficheiro de definições com o seu valor por omissão e o seu intervalo.
* [A biblioteca de materiais](docs/materials.md): cada chave de um material e como um material é aplicado

## 🤝 Contribuir e comunidade

* Antes de um pull request execute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tudo tem de passar. Veja [CONTRIBUTING.md](CONTRIBUTING.md) e o [código de conduta](CODE_OF_CONDUCT.md).
* O README existe em oito idiomas: uma alteração ao texto faz-se nos oito ficheiros, e as duas imagens são desenhadas a partir de `docs/architecture.json` e `docs/structure.json` por `tools/make_architecture.py` e `tools/make_structure.py`.
* Comunique uma vulnerabilidade em privado: [SECURITY.md](SECURITY.md). Perguntas, ideias e relatos: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.
