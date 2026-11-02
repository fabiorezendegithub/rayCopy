# rayCopy — spec de integração

App Tauri 2 (Rust), UI estática em `ui/` (HTML/CSS/JS puro, sem bundler).
Port de https://ray.so: cria imagens bonitas de código.

## Contrato (src/model.rs)

Snapshot enviado pela UI a cada comando (JSON camelCase):

```json
{
  "code": "string",
  "language": "auto | csharp | html | css | javascript",
  "title": "string",
  "subtitle": "string",
  "theme": "candy | sand",
  "dark": false,
  "background": true,
  "lineNumbers": false,
  "padding": 64,            // 16 | 32 | 64 | 128
  "font": "jetbrains-mono",  // ver FONT_IDS em model.rs
  "highlightedLines": [2, 3],
  "width": null,            // px ou null (automático)
  "scale": 4,               // 2 | 4 | 6
  "windowChrome": true
}
```

## Comandos Tauri (invoke)

Com `withGlobalTauri`, use `window.__TAURI__.core.invoke(name, args)`.

| Comando | Args | Retorno |
|---|---|---|
| `highlight` | `{ snap }` | `Token[][]` (uma lista de tokens por linha) |
| `detect_language` | `{ code }` | `"csharp" \| "html" \| "css" \| "javascript"` |
| `format_code` | `{ code, language }` | string formatada |
| `export_png` | `{ snap }` | caminho salvo ou `null` se cancelado |
| `export_svg` | `{ snap }` | caminho salvo ou `null` se cancelado |
| `copy_png` | `{ snap }` | `null` |

`Token = { text, color, italic }`.

## Responsabilidades

- `src/theme.rs` — paletas Candy/Sand, claro/escuro (pronto).
- `src/highlight.rs` — syntect, só C#, HTML, CSS, JavaScript. Detecção automática.
- `src/render.rs` — gera SVG da moldura (janela, gradiente, linhas, números, título) e rasteriza em PNG com resvg.
- `src/commands.rs` — comandos Tauri, diálogos de salvar (rfd), clipboard (arboard).
- `ui/` — layout e controles idênticos ao ray.so, estado persistido no hash da URL.

Não edite arquivos de outro dono. Se precisar de mudança no contrato, avise o chief of staff.
