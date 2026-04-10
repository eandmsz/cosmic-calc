# COSMIC Calculator
Calculator for the COSMIC desktop

- Native Rust applicaiton. No dependency on any other language (no wrappers for C or Python)
- Focusing on simplicity, ease of use, touch friendly operation, logical and aesthetic layout
- IEEE 754 double-precision (64-bit) floating-point precision
- Stateful operation for more intuitive workflow

# Features:
- Repeat last operation using =
- Predictable operation: only the = sign evaluates the expressions
- Intuitive Backspace and AC/C functions
- Automatic scientific mode in landscape window
- Easily readable expressions with superscript, subscript
- Customizable Rand function using xoshiro256** fast pseudorandom number generator known from its excellent statistical quality
- Trigonometry and radical functions work both before or after inputting an operand
- Fully compatible with iOS/macOS ASCII expressions e.g:
	- 1-2×-5×4,5e3÷1000
	- √(sin^-1(1)-tan^-1(1))×∛8×root(16, 4)×π×𝑒
- Also compatible with other ASCII expression formats:
	- 1-2 * -5 *  4.5E3/1000
	- sqrt(asin(1)-atan(1))cbrt(8)rOOt(16, 4)pi*e
- IEEE 754 standard implemented with native f64 variables to get a high performance and lightweight application
	- 1-bit sign (+/-)
	- 11-bits exponent (limited to 10^307 to be reliable)
	- 52-bits mantissa (fraction) +1 implicit bit which gives 15.95 digits precision (rounded down to a reliable 15 digits)
- Indicates in real-time if the user input or calculation result is a prime or not:
 	- The Miller-Rabin primality test with 7 deterministic bases {2,3,5,7,11,13,17} is mathematically proven to give a 100% accurate prime number detection up to 2^64 (~10^19) and since this calculator has 15 digits precision it's sufficient

# Will not be suppored - Out of scope for a simple calculator:
- Integral, derivative, lim, combinations (nCr), permutations (nPr), Fibonacci function
- Complex numbers and their imaginary units (negative number under sqrt will give an error instead)
- Programmer's operations: bitshift, binary, hexadecimal calcualtions
- Economic and statistics calculations: mean, standard deviation, sum of squares (use a spreadsheet for that)
- Graphing calculations (use a spreadsheet for that)
- Date, Currency, Unit conversions (currency would need a data provider and we want to keep this tool to be 100% offline)
- Area, perimeter, volume, surface formulas
- Physics, chemistry formulas/constants

# Video demonstrations:
