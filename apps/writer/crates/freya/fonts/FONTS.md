# Writer fonts

Writer embeds these OFL 1.1 families with `include_bytes!` and registers them at launch
(`LaunchConfig::with_font` in `src/main.rs`), so the chains in `src/design/tokens.rs` resolve
without system fonts. Each family's licence is `OFL.txt` in its folder; packages install them under
`licenses/fonts/`.

| file                                                                          | registered as      | version | source                                                         | axes                                  |
| ----------------------------------------------------------------------------- | ------------------ | ------- | -------------------------------------------------------------- | ------------------------------------- |
| `fraunces/Fraunces72ptSoft-SemiBold.ttf`                                      | Fraunces 72pt Soft | 1.000   | github.com/undercasetype/Fraunces release 1.000 (static)       | opsz 72, SOFT 50, wght 600            |
| `fraunces/Fraunces-Variable.ttf`                                              | Fraunces           | 1.000   | google/fonts `ofl/fraunces/Fraunces[SOFT,WONK,opsz,wght].ttf`  | default instance                      |
| `literata/Literata-Variable.ttf`, `Literata-Italic-Variable.ttf`              | Literata           | 3.103   | github.com/googlefonts/literata release 3.103 `fonts/variable` | default instance (opsz 12, wght 400)  |
| `ibm-plex-sans/IBMPlexSans-{Regular,Italic,Medium,MediumItalic,SemiBold}.ttf` | IBM Plex Sans      | 3.005   | github.com/IBM/plex `@ibm/plex-sans@1.1.0`                     | static 400, 500, 600; italic 400, 500 |
| `ibm-plex-mono/IBMPlexMono-{Regular,Italic,Medium}.ttf`                       | IBM Plex Mono      | 2.005   | github.com/IBM/plex `@ibm/plex-mono@2.5.0`                     | static 400, 500; italic 400           |

Freya renders a variable file at its default instance whatever weight is requested, so display type
uses the static Fraunces cut and Plex ships the static weights the UI uses. The variable Fraunces
stays as the next fallback. The upstream Fraunces release zip has no licence file; the project's OFL
text comes from google/fonts.

Fraunces italic is not shipped because no display text in Writer is italic.

## Scripts

The bundled faces cover Latin. Literata and IBM Plex Sans also cover Cyrillic and Greek; IBM Plex
Mono covers Cyrillic, and Greek in mono text falls back. Fraunces is Latin-only, so display titles
in other scripts fall through to Literata. Chinese, Japanese, Korean, Arabic, Hebrew, Devanagari and
Thai rely on system fonts, which are too large to bundle. The chains in `src/design/tokens.rs` name
them before the generic family:

- macOS: PingFang SC, Hiragino Sans, Apple SD Gothic Neo, Geeza Pro, Kohinoor Devanagari, Thonburi;
  serif text prefers Songti SC and Hiragino Mincho ProN.
- Linux: Noto Sans CJK SC/JP/KR, Noto Sans Arabic, Hebrew, Devanagari and Thai, and their Noto Serif
  counterparts for prose.

Skia also falls back per glyph through the system font manager, so a script whose family is not
named still renders when the system has a face for it. Han characters use the first CJK face
present, which is the Simplified Chinese one, because Writer text does not yet carry a language.
