# rayCopy

Crie imagens bonitas do seu código. Um port desktop do [ray.so](https://ray.so), feito com **Rust + Tauri 2**.

## Funcionalidades

- Syntax highlighting para **C#, HTML, CSS e JavaScript**, com detecção automática da linguagem
- Temas **Candy** e **Sand**, nos modos claro e escuro
- Janela estilo macOS, fundo em gradiente, título e subtítulo
- Números de linha e destaque de linhas (ex.: `2-4,7`)
- Padding (16, 32, 64, 128), fontes e largura configuráveis
- Formatação de código (Format Code)
- Exportação em **PNG** (2x, 4x ou 6x) e **SVG**, ou cópia da imagem para a área de transferência
- Estado salvo no hash da URL, com opção de copiar o link

## Pré-requisitos

- [Rust](https://rustup.rs) (com `cargo`)
- No Linux, WebKitGTK 4.1 e GTK 3:

  ```bash
  # Debian / Ubuntu
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev build-essential pkg-config

  # Arch / Omarchy
  sudo pacman -S webkit2gtk-4.1 gtk3 libsoup3 base-devel pkg-config
  ```

- No Windows e no macOS não é preciso nada além do Rust (o Tauri usa WebView2 e WebKit nativos).

Não precisa de Node.js, npm nem da CLI do Tauri.

## Como usar

```bash
git clone <url-do-repositorio>
cd rayCopy
cargo run
```

Versão de release:

```bash
cargo build --release
./target/release/raycopy
```

Testes:

```bash
cargo test
```

Mais detalhes, incluindo problemas comuns, em [docs/como-executar.md](docs/como-executar.md).

## Estrutura

```
rayCopy/
├── Cargo.toml          dependências Rust
├── tauri.conf.json     configuração do Tauri
├── build.rs            integração com o tauri-build
├── src/
│   ├── main.rs         registra os comandos Tauri
│   ├── commands.rs     highlight, format, exportação e clipboard
│   ├── highlight.rs    syntax highlighting (syntect)
│   ├── render.rs       gera o SVG e rasteriza para PNG (resvg)
│   ├── theme.rs        paletas Candy e Sand
│   └── model.rs        contrato de dados (Snapshot, Token)
├── ui/                 interface em HTML, CSS e JS puro
├── docs/               documentação
└── SPEC.md             contrato entre a UI e o backend
```

## Tecnologias

[Tauri 2](https://tauri.app) · [syntect](https://github.com/trishume/syntect) · [resvg](https://github.com/linebender/resvg) · [rfd](https://github.com/PolyMeilex/rfd) · [arboard](https://github.com/1Password/arboard)

## Créditos

Inspirado no [ray.so](https://ray.so) da Raycast.
