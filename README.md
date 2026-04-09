# COSMIC Calculator
Calculator for the COSMIC desktop

- Native Rust applicaiton: No dependency on any other language (no wrappers for C or Python modules)
- Focusing on simplicity, ease of use, logical button layout, touch friendly operation: We have identified many features missing from other Linux calculators
- IEEE 754 double-precision floating-point precision: We want to keep it fast and lightweight and having more precision wouldn't make pracital sense

# Features:
- Repeat last operation using =
- Predictable operation: only the = sign evaluates the expressions
- Intuitive Backspace and AC/C functions
- Automatic scientific mode in landscape window
- Easily readable expressions with superscript, subscript
- Trigonometry and radical functions work both before or after inputting an operand
- Copy/Paste expressions and results (iOS, macOS Calculator compatible ASCII formatting + additional compatility)
- Customizable random function (from GUI)
- Customizable button layout (from config file)
- Native f64 Rust variables to get the highest performance supported by x64 processors while also keeping the code simple:
	- 1-bit sign (+/-)
	- 11-bits exponent (limited to 10^307 to be reliable)
	- 52-bits mantissa (fraction) +1 implicit bit which gives 15.95 digits precision (rounded down to a reliable 15 digits)
- User input or calculator result indicates if it's a prime number or not:
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
