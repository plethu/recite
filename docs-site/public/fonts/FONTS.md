# Site fonts

Latin subsets of the OFL families, converted to variable woff2 with `fonttools subset`
(`--flavor=woff2 --layout-features='*'`) and the unicode range in `src/styles/fonts.css`. The
licence for each family is in `licenses/`.

| file | family | version | source |
| --- | --- | --- | --- |
| `fraunces-latin.woff2` | Fraunces (opsz, wght, SOFT, WONK) | 1.000 | google/fonts `ofl/fraunces` |
| `literata-latin.woff2` | Literata (opsz, wght) | 3.103 | github.com/googlefonts/literata, release 3.103 |
| `ibm-plex-sans-latin.woff2` | IBM Plex Sans Var (wght, wdth) | 3.000 | github.com/IBM/plex, `@ibm/plex-sans-variable@0.2.0` |
| `ibm-plex-mono-latin.woff2` | IBM Plex Mono Var (wght) | 1.000 | github.com/IBM/plex, `@ibm/plex-mono-variable@1.0.0` |

Each family ships separate `-latin`, `-cyrillic` and `-greek` files where the source covers the
script (Literata upright and italic, IBM Plex Sans; IBM Plex Mono Latin and Cyrillic only), so a page
downloads only the scripts it uses. Fraunces is Latin-only. Chinese, Japanese, Korean, Arabic, Hebrew,
Devanagari and Thai fall through to system faces.
