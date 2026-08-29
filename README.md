# COSMIC Calculator

A scientific calculator for the COSMIC desktop.

- Native Rust application. No dependency on any other language (no wrappers for C or Python)
- Focusing on simplicity, ease of use, touch friendly operation, logical and aesthetic layout
- Decimal arithmetic, so `0.1 + 0.2 - 0.3` is 0 and `0.3 mod 0.1` is 0
  — the numbers you type are the numbers it adds
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

- **`core/`** — `cosmic-calc-core`: the decimal number type, the
  tokenizer, parser, evaluator, display formatter, configuration,
  themes, locale handling, clipboard sanitising, history and memory.
  No GUI dependencies.
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
- Intuitive Backspace and AC/C functions. `C` takes back the last
  operand — the number, the bracketed group or the whole function call
  you just finished — and leaves the expression it was part of
  standing; with an operator under the cursor there is nothing to take
  back, so the press only flips the key to `AC`, which clears the
  display. Either way one `C` arms the `AC`, so the line is never more
  than two presses from empty
	- Neither key ever leaves the calculator's own work on screen. A
	  `×` it inserted for you goes when the operand it was
	  multiplying goes, so backspacing the `(2)` out of `5×(2)`
	  leaves `5` and not `5×`
	- What one press wrote, one press takes back: `x²` on `2^2`
	  writes `(2^2)²`, and one backspace gives the `2^2` back rather
	  than a `(2^2)` you did not ask for or an empty exponent slot
	  waiting for a digit
	- The two keys part company over a fixed exponent, because they
	  are asking different questions. Backspace takes back the press,
	  so `5²` gives the `5` back; `C` takes back the value, and `5²`
	  is one value, so it goes whole — `5+2³` then `C` is `5+`
	- A two-argument call unwinds in the one order it is filled in.
	  `logᵧ 98 ) 71` reads log₇₁(98), and backspace takes it apart
	  the way it went together, last piece first: into the base, out
	  through its digits, back inside the brackets to the argument,
	  out through its digits, and only with both slots empty does
	  the call itself come off. `³√(8)` is the same walk through the
	  degree and the radicand
	- Backspace retraces a cursor the keypad moved. `yˣ` parks it in
	  front of what you typed and `logᵧ`/`ʸ√x` park it in a slot, and
	  in each case backspace takes the press back and returns the
	  cursor to where that press found it — where it used to stick
	  with nothing to its left to delete
	- Neither argument can be deleted out from under a call, so a
	  `log₇₁(98)` cannot become `log(7198)` — log base ten of a
	  number you never typed — and a `³√(8)` cannot become a `√(83)`
- Easily readable expressions with superscript exponents, subscript log
  bases and the radical sign: `2⁵`, `3×10⁴`, `2¹·⁵`, `log₂(8)`,
  `sin⁻¹(1)`, `³√(8)`, `⁴√(16)`. The `^` never reaches the display —
  the raising is what it says
	- Every root is the one radical with its degree written into it,
	  the cube root included: `³√(8)` on the display and on the key,
	  rather than the single `∛` glyph a good many fonts do not carry
	  and draw as a box. The buffer and the clipboard still say
	  `cbrt(8)`
	- Both the square and cube root keys wear the whole operation —
	  `²√x` and `³√x`, the way `ʸ√x` does with its degree left
	  open — rather than a bare sign that says which radical it is
	  but not what it does to what is on screen
	- The keys are drawn the same way as what they write. A script on
	  a button face is the key's own font at 60%, placed one step off
	  the line, rather than a raised or lowered glyph asked of the
	  font: Unicode has one for the `2` of `x²` but only borrowed
	  letters for the `ˣ` of `2ˣ` and a Greek gamma for the `ᵧ` of
	  `logᵧ`, and those came out at whatever height and weight the
	  face that happened to carry them drew. Placed instead of found,
	  every exponent on the keypad sits at the same height
	- What goes up is the text itself, drawn smaller and moved off the
	  line, rather than a superscript glyph swapped in for it. So
	  anything can go up: a decimal separator (`2¹·⁵` in either
	  locale), a factorial (`2^2!` reads as `2` with a small raised
	  `2!`, never as `2 × 2!`), a whole call — `2^sin(30)` raises the
	  `sin(30)` and stays legible as a power. A script inside a script
	  steps again, so `2^3^2` shows its `2` smaller and higher still
	- A slot you have opened but not filled shows as small brackets —
	  `2⁽⁾` for a power key pressed before its exponent, `()²` for the
	  base `yˣ` is waiting for, `log₍₎(8)` for a base — and they are
	  drawn dim while the cursor is in them, which is what says the
	  next digit lands there rather than after the call
	- They stay up once you have typed into the slot, round what is
	  there rather than in place of it, for as long as the cursor is
	  still in it: `2⁽¹⁵⁾` while the exponent is being keyed, with
	  the opener lit now that the slot has been reached and the
	  closer dim like every closer you are inside. One digit does not
	  mean you are finished with a number, and the display draws no
	  cursor of its own, so nothing else on screen would say the next
	  digit lands up there too. They go when you leave the slot — `)`
	  from a `logᵧ` base or a root degree steps out of the call, and
	  `)` on a power brackets it, so either way the finished `2⁵`
	  reads as one. A slot you opened a bracket of your own at the
	  head of, or filled with a call, wears no second pair: that one
	  is real, and it stays
	- Three levels is as deep as it goes: `2^2^2` is drawn in full, and
	  a key that would write a fourth puts what is already there in
	  brackets and starts again from the line — `x²` on `2^2^2` gives
	  `(2^2^2)²`, the same number one level shallower. `logᵧ` and `ʸ√x`
	  do it without adding brackets, since a call carries its own:
	  `logᵧ` on `2^2^8` reads `log₍₎(2^2^8)`. Where nothing can make
	  room — a base already two steps under the line, or `yˣ`, which
	  takes its operand *up* a level — the press does nothing rather
	  than draw a script too small to read. `yˣ` counts the whole
	  power it would raise, not just the operand under the cursor:
	  pressed on the `4` of `4^3^2` it would move all three levels up
	  and is refused
	- The caption above the display is the same row of sized pieces the
	  display below it is, so an expression reads the same in both.
	  History rows are single lines of one size, and fall back there to
	  Unicode's raised and lowered glyphs, and to brackets where those
	  run out: `2⁽2!⁾`, `2⁽2²⁾`
	- The "Show ASCII expression" toggle in the settings panel switches
	  the display to the text Copy would put on the clipboard, which is
	  what the tokenizer is handed either way: `2^5`, `log2(8)`,
	  `sin-1(1)`, `root(16,4)`, `pi*e`, `sqrt(9)/cbrt(8)`, `1234.5`.
	  All of it ASCII, down to the spelled-out constants, the `sqrt(`
	  the radical stands for, and a number written without the
	  thousands separators the display groups it with — the notation
	  changes, the value never does
- `logᵧ` writes its base where a base belongs — under the log — and
  shows the empty slot until you type one: press it and the display
  reads `log₍₎(8)`, key the base and it reads `log₂(8)`. With an
  operand already typed the press goes straight to the base (`8`,
  `logᵧ`, `2` = 3), and from an empty display the order is the same
  one rather than the mirror of it: the argument first, then `)` out
  to the base (`logᵧ`, `8`, `)`, `2` = 3), with a second `)` leaving
  the call. One order whatever is on screen when you reach the key,
  which is also the order backspace unwinds it in
- `ʸ√x` writes its degree where a degree belongs — in the opening of
  the sign rather than beside it, the way `⁴√` is printed as one symbol
  rather than a small 4 standing next to a stroke — and reads the
  same way round: `16`, `ʸ√x`, `4` gives `⁴√(16)`. From an empty
  display it is the same order again — the radicand first, then `)`
  out to the degree (`ʸ√x`, `8`, `)`, `3` = 2), with a second `)`
  leaving the call, exactly as `logᵧ` does with its base
- `)` with nothing open to close brackets the operand you just typed
  instead of doing nothing: `5+2` then `)` reads `5+(2)`. Where a
  bracket *is* open the key closes it as before, stepping over the
  closer the `(` key already wrote
	- In a script slot it closes the *slot*, and writes nothing: an
	  exponent typed straight after the caret is a slot the display
	  draws brackets round, and `)` is how you say you are done with
	  it. `2`, `xʸ`, `3`, `)` is `2³` — the brackets come down and
	  the next `+` lands on the line — where it used to write a pair
	  round the whole power and give `(2^3)`. Same for the base slot
	  `yˣ` opens, where the press steps the cursor out past the
	  power: `5`, `yˣ`, `6`, `)`, `+` is `6⁵+`
	- An operator still waiting for its right operand has no value
	  for the brackets to close over, so the press takes it back:
	  `(5+` then `)` is `(5)`
- Customizable Rand function, drawing from the OS entropy source
  (`getrandom`/`/dev/urandom` on Linux) so each press is independent of
  the last
- `−` where a value begins is that value's sign rather than a
  subtraction: `−`, `6` is `-6`, not the `0-6` a supplied left operand
  used to make of it. `+`, `×` and `÷` still start an empty display on
  a `0`, which is what they need and a sign does not
	- Every slot of `logᵧ` and `ʸ√x` is a place a value begins, so a
	  negative can be keyed into any of them — the argument, the
	  base, the radicand, the degree — where the press used to be
	  dropped
	- And the two calls close over the sign along with the number:
	  `−`, `4`, `logᵧ`, `8`, `)` is `log₈(-4)`, where the sign used
	  to be left outside as `-log₈(4)`, which negates the logarithm
	  instead of taking one of a negative number
- A decimal separator with no digits behind it goes when you move on
  from it: `5`, `.`, `+` is `5+`. Backspace is the one press that
  leaves it, since deleting it is what you are asking for
- Trigonometry and radical functions work both before or after inputting an operand
- Both power orders on the keypad: `xʸ` raises what you have already
  typed to what you type next (`2`, `xʸ`, `3` = 8), and `yˣ` reads the
  same two operands the other way round, making the one you have typed
  the exponent (`2`, `yˣ`, `3` = 9)
	- Both need the operand they raise, and neither invents one: on an
	  empty display — where the `0` on screen is the calculator's, not
	  yours — and after an operator or an open bracket, the press does
	  nothing. `0`, `xʸ`, `5` is `0⁵`, because that `0` you typed
	- `yˣ` puts the base slot in front of what you typed and parks the
	  cursor in it, so `2`, `yˣ` reads `()²` with the brackets dim:
	  the next digit goes under the 2, not after it. The brackets stay
	  round the base once it holds something — `5`, `yˣ`, `6` reads
	  `(6)⁵`, because that `6` is still the slot the next digit joins
	  — and come down when `)` closes the slot. Open a bracket of your
	  own in there and it is yours: it stays after you close it
	- An operator keyed while the cursor is still in that slot is
	  about the whole power, and goes after it: `2`, `yˣ`, `3`, `+`
	  reads `3²+`. To put an expression rather than a number in the
	  slot, open a bracket there and write inside it
	- `x²` and `x³` square and cube what is on screen rather than
	  adding a level to it: `2^3` then `x²` is `(2^3)²` = 64, where a
	  second caret would have said `2^3^2` = 512. Press it again and
	  the brackets nest — `((2^3)^2)²` — which is what squaring twice
	  is. `xʸ` is the key for building a tower
	- They finish the operation, so their exponent is not a slot
	  anything else can reach. `5`, `x²`, `3` is `5²×3` = 75, with
	  the `×` filled in for you, where the digit used to run onto the
	  end of the exponent and give `5` to the twenty-third. A second
	  caret is bracketed for the same reason: `5²` then `xʸ` is
	  `(5²)^y`, since the buffer spells the square `5^2` and a bare
	  `5^2^y` would raise the `2`
	- And they are about the whole value, not the piece of it the
	  cursor is parked in front of. `6`, `yˣ`, `3` reads `3⁶` with
	  the cursor still in the base slot, and `x²` there is `(3⁶)²`
	  where it used to write `3^2^6` — the 3 raised to the
	  sixty-fourth
- Fully compatible with COSMIC desktop themes and also inheriting accent color from KDE, GNOME, XFCE
- Decimal separator is automatically based on the system locale, and
  the thousands separator follows it unless you pick one. The space
  the two of them can resolve to is a no-break space, so a grouped
  number can never break across a line the display has no room for
- Fully compatible with iOS/macOS ASCII expressions e.g:
	- 1-2×-8%5×4,5e3×100÷2^2^2×((2^2)^2)^2
	- √(sin^-1(1)+tan^-1(1))×∛8×root(16, 4)×3π×2𝑒+2e3
- Also compatible with alternative formatting and characters:
	- 1-2 * −8mod5 *  4.5E3* 100/2^2^2*( ( 2^2 )^ 2) ^2*
	- sQrt(SIN−1(1)＋atan(1))cbrt(8)rOOt(16, 4)3pI*2e＋2e3+(
- Arithmetic in base ten, on a fixed-precision decimal of the kind
  Apple's calculator uses. `+`, `−`, `×`, `÷`, percent, modulo,
  whole-number powers and small factorials are all carried out in
  decimal, so a number you can write down is one it holds exactly and
  no binary representation error enters the arithmetic to begin with
	- `18` significant digits are carried and **15 are displayed**.
	  Significant digits, not digits after the point: a value carries
	  the same number of digits wherever its decimal point is
	- The three digits that are not shown are guard digits, and they
	  are what makes the rounding of a division invisible: `1÷3 = ×3 =`
	  gives back `1`, because the eighteen threes that were divided out
	  are the eighteen that get multiplied
	- Rounding the display to 15 digits already hid most of what binary
	  got wrong, and still does: `1.005 × 100` printed `100.5` under
	  doubles too, because the `100.49999999999999` they really held
	  goes back to `100.5` once the last two digits are rounded off.
	  What base ten changes is the cases where the error escapes those
	  last digits — where it is the answer rather than a nick in it:
		- **Cancellation.** Subtracting near-equal values destroys the
		  leading digits and promotes the error to the whole result:
		  `0.1 + 0.2 − 0.3` is `0`, not `5.5511151231258e-17`, and
		  `100.1 − 100` is `0.1`, not `0.0999999999999943`
		- **Remainders**, where being a hair under a multiple changes
		  the answer instead of its last digit: `0.3 mod 0.1` is `0`,
		  where binary printed a clean-looking, wrong `0.1`
		- **Sums across scales**, where 18 digits reach further than a
		  double's 15-to-17: `10000000000000000 + 1 − 10000000000000000`
		  is `1`, where a double had nowhere to put the `1` and gave `0`
	- The rounding is the display's, not the calculator's. A result
	  carried into the next calculation is used at the precision it was
	  computed at, and the fifteen digits on screen are a view of it.
	  Edit those digits and they become the number: what you can see is
	  what is computed from
- Trigonometry, logarithms, roots and fractional powers have no
  decimal algorithm worth writing, so those go out to IEEE 754
  double-precision `f64` and come back as the shortest decimal that
  identifies the answer. `√0.01` is therefore `0.1` and not
  `0.1000000000000000055`, and the arithmetic that follows it is exact
  again. The range is the double one either way: values above about
  `10^308` report Overflow and non-zero values below `10^-308` report
  Underflow
- Opens where you left it: the window size is remembered, written out a
  couple of seconds after you stop dragging the edge rather than on
  every frame of the drag. "Save window size" in the settings turns
  that off — and turning it back on records the size you are looking
  at there and then
- Side panels dock beside the calculator rather than over it, so the
  window grows to make room for them and cannot be dragged in narrower
  than the calculator plus whatever panels are open
- One `%` key for both readings, decided by what follows it: on its own
  it is a percentage (`3.5%×230` = 8.05, `200+10%` = 220), and with an
  operand straight after it, it is modulo (`5%3.2` = 1.8, `7%(-3)` = 1)
	- `%` and `!` write themselves and nothing else — neither puts
	  brackets round what it applies to. Keyed in an exponent they
	  stay in the exponent, which is how the text reads them
	  anyway: `2`, `xʸ`, `5`, `%` is `2^5%`, and `2`, `xʸ`, `3`,
	  `!` is `2` raised to `3!` = 64
- An expression with no value says which part of it has none, rather
  than a bare "Undefined": `√(-4)` is `Undefined: Negative number
  under even root`, `ln(0)` is `Undefined: 0 inside logarithm`,
  `log₁(8)` is `Undefined: Logarithm base cannot be 1`, `4÷0` is
  `Undefined: Division by 0`, `tan(90°)` is `Undefined: Tangent`,
  `cot(0)` is `Undefined: Cotangent`, `sin⁻¹(5)` is `Undefined
  sin−1(x) must be between −1 and 1`, and `0^-2` — which used to
  report Overflow, saying the answer was too big rather than that
  there is none — is `Undefined: 0 raised to negative power`. The
  cases with no name of their own still read `Undefined`
- The memory register sits under the display, at the size and in the
  colours of the number-property labels and aligned to the right: a
  dim `M` while nothing is stored, the value beside it once something
  is. It used to be a line at the top of the history panel, where it
  could only be read with that panel open
- Every on/off setting is one block at the top of the settings panel,
  each on its own line with the switch against the right edge — show
  result properties, show memory, save window size, save history, show
  ASCII expression. Theme and font go last, since they are the two
  longest controls and the two you set once
- "Save history" keeps the history list in `config.toml` and reads it
  back on the next start, updated as each calculation is recorded.
  Turning it off empties it from the file straight away; turning it on
  writes what is already on screen. Entries are stored as the ASCII
  expression the clipboard would carry, so the file stays readable and
  a row can be clicked back into the buffer after a restart
- Shows real-time number properties in both layouts: prime ; harshad ; palindrome ; square ; triangular ; fibonacci
 	- Miller-Rabin primality test is used with 9 deterministic bases which gives a fast and 100% accurate prime number detection up to 2^64 (~10^19)

## Customising the keypad

A first run — before there is a `config.toml` — opens on the Basic
keypad; the button in the middle of the top bar switches to Scientific
and back, and whichever one you leave it on is the one it opens on next
time.

The keypad is laid out from `config.toml`
(`~/.config/cosmic-calc/config.toml`). The grid size is fixed — Basic
is 4 columns × 5 rows, Scientific is 9 columns × 5 rows — but every
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
    "second lparen rparen e ee clear backspace percent div",
    "square cube xpowy epowx tenpowx 7 8 9 mul",
    "sqrt cbrt yrootx ln log 4 5 6 sub",
    "factorial sin cos tan pi 1 2 3 add",
    "rand sinh cosh tanh reciprocal negate 0 decimal equals",
]
scientific_second = [
    "second lparen rparen e ee clear backspace percent div",
    "square cube xpowy ypowx twopowx 7 8 9 mul",
    "sqrt cbrt yrootx logy log2 4 5 6 sub",
    "factorial asin acos atan pi 1 2 3 add",
    "rand asinh acosh atanh reciprocal negate 0 decimal equals",
]
```

`basic_second` exists too, and starts out identical to `basic` — put a
`second` key on the Basic keypad and it becomes the layout that key
switches to.

The Scientific keypad ships with all nine columns filled. A
`config.toml` written while its leftmost column was still empty keeps
working — those rows are one cell short, so the blank lands on the
right-hand end instead. Delete the `[keypad]` section to take the
shipped layout back.

Because the two tables are independent, the `2nd` key toggles exactly
what you decide. In the shipped Scientific layout it turns over only
the ten keys that have something to turn into: `sin`/`cos`/`tan` and
the hyperbolics become their inverses, `ln` becomes `logy`, `log`
becomes `log2`, `epowx` becomes `ypowx` and `tenpowx` becomes
`twopowx`. Everything else — `xpowy`, `yrootx`, `sqrt`, `pi`, the
digits — sits in both tables and holds still under your fingers.

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
| Powers and roots | `square`, `cube`, `xpowy`, `ypowx`, `sqrt`, `cbrt`, `yrootx` |
| Constants | `pi`, `e` |
| Other | `reciprocal`, `rand`, `mc`, `mr`, `m+`, `m-` |
| App | `mode`, `angle`, `history`, `settings` |

Common spellings are accepted as aliases, so `x^2`, `x2` and `square`
are the same key, as are `π`/`pi`, `2nd`/`second`, `1/x`/`reciprocal`,
`+/-`/`±`/`negate` and `%`/`percent`.

## Out of scope for a simple calculator

- Arbitrary "infinite" precision arithmetic (the decimal type is a
  fixed 18 digits — enough that the 15 on screen are always right, not
  enough to hold a number of any size you like)
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
