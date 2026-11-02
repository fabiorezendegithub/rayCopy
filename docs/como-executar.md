# Como executar o rayCopy

O rayCopy é um aplicativo desktop feito com **Rust + Tauri 2**. A interface fica em `ui/` (HTML, CSS e JavaScript puro) e o backend em `src/` (Rust).

## 1. Pré-requisitos

- **Rust** (com `cargo`), instalado pelo [rustup](https://rustup.rs). Testado com Rust 1.99.
- **Dependências do sistema para Tauri no Linux** (WebKitGTK 4.1 e GTK 3):

  Debian / Ubuntu:
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev build-essential pkg-config
  ```

  Arch / Omarchy:
  ```bash
  sudo pacman -S webkit2gtk-4.1 gtk3 libsoup3 base-devel pkg-config
  ```

  Você pode conferir se estão instaladas com:
  ```bash
  pkg-config --modversion webkit2gtk-4.1 gtk+-3.0 libsoup-3.0
  ```
  Se algum comando não imprimir a versão, a dependência está faltando.

- No Windows e no macOS, o Tauri usa WebView2 e WebKit nativos. Não é preciso instalar nada extra além do Rust.

Não é necessário Node.js, npm nem a CLI do Tauri. O projeto usa `cargo` diretamente.

## 2. Executar em modo de desenvolvimento

Na raiz do projeto:

```bash
cd /home/fabio/dev/rayCopy
cargo run
```

Na primeira vez o cargo baixa e compila as dependências, o que pode levar alguns minutos. As execuções seguintes são rápidas.

A janela do rayCopy abre com um exemplo de código já carregado.

## 3. Gerar a versão de release

```bash
cargo build --release
```

O executável fica em `target/release/raycopy`. Para rodar:

```bash
./target/release/raycopy
```

## 4. Rodar os testes

```bash
cargo test
```

Devem passar 13 testes, cobrindo detecção de linguagem, highlight, formatação, renderização do SVG/PNG e sanitização do nome do arquivo.

## 5. Usando o aplicativo

1. Cole ou digite o código no editor.
2. Ajuste título, subtítulo, idioma (Auto-Detect, C#, HTML, CSS ou JavaScript), tema (Candy ou Sand), modo escuro, fundo, números de linha, padding, fonte, janela, largura e linhas em destaque (ex.: `2-4,7`).
3. Use **Format Code** para formatar o código, inclusive quando ele estiver compactado em uma única linha.
4. Exporte com o botão **Export as PNG**, ou pelo menu de opções:
   - **Save PNG** / **Save SVG**: abre o diálogo para escolher onde salvar.
   - **Copy Image**: copia a imagem para a área de transferência.
   - **Copy URL**: copia um link com o estado atual.
   - **Size**: escolhe 2x, 4x ou 6x.

As configurações são salvas no hash da URL e o tamanho de exportação fica no `localStorage`.

## 6. Problemas comuns

| Sintoma | Causa provável | Solução |
|---|---|---|
| `The system library 'webkit2gtk-4.1' required by crate ... was not found` | Dependências do sistema não instaladas | Instale os pacotes da seção 1 |
| A janela abre, mas fica em branco | Caminho do frontend errado | Confirme que `tauri.conf.json` tem `"frontendDist": "ui"` e que a pasta `ui/` existe |
| Fontes diferentes das esperadas | As fontes não vêm embutidas no app | Instale a fonte no sistema ou aceite a monoespaçada padrão |
| Copy Image ou Save falham | Erro específico do ambiente gráfico | A mensagem de erro aparece no toast da interface; anote-a para investigar |

## 7. Estrutura do projeto

```
rayCopy/
├── Cargo.toml          dependências Rust
├── tauri.conf.json     configuração do Tauri (janela, frontend, ícone)
├── build.rs            integração com o tauri-build
├── src/
│   ├── main.rs         registra os comandos Tauri
│   ├── commands.rs     highlight, format, exportação e clipboard
│   ├── highlight.rs    syntax highlighting (C#, HTML, CSS, JavaScript)
│   ├── render.rs       gera o SVG da moldura e rasteriza para PNG
│   ├── theme.rs        paletas Candy e Sand (claro e escuro)
│   └── model.rs        contrato de dados (Snapshot, Token)
├── ui/                 interface (index.html, styles.css, app.js)
├── docs/               esta documentação
└── SPEC.md             especificação de integração entre UI e backend
```
