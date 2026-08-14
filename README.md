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

- Repeat last operation using =
- Predictable operation: only the = sign evaluates the expressions
- Intuitive Backspace and AC/C functions
- Automatic scientific mode in landscape window
- Easily readable expressions with superscript, subscript
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
- Shows real-time number properties: prime ; harshad ; palindrome ; square ; triangular ; fibonacci
 	- Miller-Rabin primality test is used with 9 deterministic bases which gives a fast and 100% accurate prime number detection up to 2^64 (~10^19)

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
