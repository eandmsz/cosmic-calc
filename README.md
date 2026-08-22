# COSMIC Calculator

A scientific calculator for the COSMIC desktop.

- Native Rust application. No dependency on any other language (no wrappers for C or Python)
- Focusing on simplicity, ease of use, touch friendly operation, logical and aesthetic layout
- IEEE 754 double-precision (64-bit) floating-point arithmetic
- Stateful operation for a more intuitive workflow

## Building and running

Needs a Rust toolchain and the libraries libcosmic links against
(`libxkbcommon`, `wayland`).

```sh
cargo run --release          # build and launch
make install                 # install to /usr/local (PREFIX= to change)
```

The `install` target places the binary, the `.desktop` entry and the
AppStream metainfo so the calculator shows up in the application
launcher.

## Working on it

The code is a two-crate workspace:

- **`core/`** — `cosmic-calc-core`: the tokenizer, parser, evaluator,
  display formatter, configuration, themes, locale handling, clipboard
  sanitising, history and memory. No GUI dependencies.
- **`src/`** — `cosmic-calc`: the libcosmic application, keypad, display
  and side panels, on top of the core.

That split matters for the test loop. The core builds and tests in
seconds; the full workspace has to compile libcosmic and wgpu, which
takes minutes on a cold cache. When you are working on the arithmetic:

```sh
cargo test -p cosmic-calc-core     # seconds
make check                         # fmt + clippy + the whole workspace
```

## Features

- Fully configurable keypad: every key position in both layouts, and
  what the `2nd` key toggles, is a table in `config.toml`
- Repeat last operation using =
- Predictable operation: only the = sign evaluates the expressions
- Intuitive Backspace and AC/C functions
- Automatic scientific mode in landscape window
- Easily readable expressions with superscript exponents and subscript
  log bases: `2⁵`, `3×10⁴`, `log₂(8)`, `sin⁻¹(1)`. A debug toggle in the
  settings panel switches the display back to the raw form the buffer
  stores (`2^5`, `log2(8)`, `sin-1(1)`) — an exponent that has no
  superscript spelling, such as the `2!` in `2^2!`, stays raw either way
  rather than being rendered as something that reads differently
- Customizable Rand function, drawing from the OS entropy source
  (`getrandom`/`/dev/urandom` on Linux) so each press is independent of
  the last
- Trigonometry and radical functions work both before or after inputting an operand
- Fully compatible with COSMIC desktop themes and also inheriting accent color from KDE, GNOME, XFCE
- Decimal separator is automatically based on the system locale
- Fully compatible with iOS/macOS ASCII expressions e.g:
	- 1-2×-8%5×4,5e3×100÷2^2^2×((2^2)^2)^2
	- √(sin^-1(1)+tan^-1(1))×∛8×root(16, 4)×3π×2𝑒+2e3
- Also compatible with alternative formatting and characters:
	- 1-2 * −8mod5 *  4.5E3* 100/2^2^2*( ( 2^2 )^ 2) ^2*
	- sQrt(SIN−1(1)＋atan(1))cbrt(8)rOOt(16, 4)3pI*2e＋2e3+(
- IEEE 754 implemented with native f64 to keep the application fast, light and the codebase simple
	- 1-bit sign (+/-)
	- 11-bit exponent (limited to 10^307 to stay reliable)
	- 52-bit mantissa (fraction) + 1 implicit bit, giving 15.95 digits of
	  precision — so results are displayed to **15 significant digits**.
	  Significant digits, not digits after the point: an f64 cannot back
	  15 decimals *and* an integer part, and rounding as though it could
	  is what makes other calculators print things like
	  `8.2 + 8.2 = 16.399999999999999`.
- One `%` key for both readings, decided by what follows it: on its own
  it is a percentage (`3.5%×230` = 8.05, `200+10%` = 220), and with an
  operand straight after it, it is modulo (`5%3.2` = 1.8, `7%(-3)` = 1)
- Shows real-time number properties in both layouts: prime ; harshad ; palindrome ; square ; triangular ; fibonacci
 	- Miller-Rabin primality test is used with 9 deterministic bases which gives a fast and 100% accurate prime number detection up to 2^64 (~10^19)

## Customising the keypad

The keypad is laid out from `config.toml`
(`~/.config/cosmic-calc/config.toml`). The grid size is fixed — Basic
is 4 columns × 5 rows, Scientific is 8 columns × 5 rows — but every
cell in it is yours to assign. Each layout has two tables: the one
drawn normally, and the one the `2nd` key switches to. One line per
keypad row, naming its cells left to right:

```toml
[keypad]
basic = [
    "clear backspace percent div",
    "7 8 9 mul",
    "4 5 6 sub",
    "1 2 3 add",
    "negate 0 decimal equals",
]
scientific = [
    "second sin cos tan clear backspace percent div",
    "pi sinh cosh tanh 7 8 9 mul",
    "cube ln log log2 4 5 6 sub",
    "lparen rparen square xpowy 1 2 3 add",
    "rand ee factorial reciprocal negate 0 decimal equals",
]
scientific_second = [
    "second asin acos atan clear backspace percent div",
    "e asinh acosh atanh 7 8 9 mul",
    "cbrt epowx tenpowx logy 4 5 6 sub",
    "lparen rparen sqrt yrootx 1 2 3 add",
    "rand ee factorial reciprocal negate 0 decimal equals",
]
```

`basic_second` exists too, and starts out identical to `basic` — put a
`second` key on the Basic keypad and it becomes the layout that key
switches to.

Because the two tables are independent, the `2nd` key toggles exactly
what you decide: cell (2, 1) of the shipped Scientific layout is `pi`
normally and `e` with `2nd` on, and `log2` turns into `logy` rather
than into an inverse of itself.

Rules of the file:

- `_` leaves a cell empty (`-` is taken — it names the minus key).
- Rows that are too short are padded, rows that are too long are
  trimmed, and a name that isn't in the list below leaves that cell
  empty and prints a warning. The grid never changes size, so a typo
  costs you one key, not the whole layout.
- Delete a table entirely (or the whole `[keypad]` section) to get the
  shipped one back on the next start.
- Keep a `second` key in both tables of a layout — if the one you can
  reach it from has it and its twin doesn't, it is put back for you, or
  the keypad would be stuck in its second function.

Key names, one per action:

| Group | Names |
| --- | --- |
| Digits | `0` … `9`, `decimal` |
| Entry | `negate`, `backspace`, `clear`, `equals`, `second` |
| Operators | `add`, `sub`, `mul`, `div`, `pow`, `percent`, `mod`, `factorial`, `ee` |
| Brackets, cursor | `lparen`, `rparen`, `left`, `right`, `home`, `end` |
| Trigonometry | `sin`, `cos`, `tan`, `asin`, `acos`, `atan` |
| Hyperbolic | `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh` |
| Logarithms | `ln`, `log`, `log2`, `logy`, `epowx`, `tenpowx`, `twopowx` |
| Powers and roots | `square`, `cube`, `xpowy`, `sqrt`, `cbrt`, `yrootx` |
| Constants | `pi`, `e` |
| Other | `reciprocal`, `rand`, `mc`, `mr`, `m+`, `m-` |
| App | `mode`, `angle`, `history`, `settings` |

Common spellings are accepted as aliases, so `x^2`, `x2` and `square`
are the same key, as are `π`/`pi`, `2nd`/`second`, `1/x`/`reciprocal`
and `%`/`percent`.

## Out of scope for a simple calculator

- Arbitrary "infinite" precision arithmetic
- Integral, derivative, lim, combinations (nCr), permutations (nPr), Fibonacci function
- Complex numbers and their imaginary units (negative number under sqrt will give an error instead)
- Programmer's operations: bitshift, binary, hexadecimal calculations
- Economic and statistics calculations: mean, standard deviation, sum of squares (use a spreadsheet for that)
- Graphing calculations (use a spreadsheet for that)
- Date, Currency, Unit conversions (currency would need a data provider and we want to keep this tool to be 100% offline)
- Area, perimeter, volume, surface formulas
- Physics, chemistry formulas/constants

## License

GPL-3.0-only. See [LICENSE](LICENSE).
