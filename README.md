# COSMIC Calculator
Calculator for the COSMIC desktop

- Native Rust applicaiton: No dependency on any other language (no wrappers for C or Python modules)
- Focusing on simplicity, ease of use, logical button layout, touch friendly operation: We have identified many features missing from other Linux calculators
- IEEE 754 double-precision floating-point precision: We want to keep it fast and lightweight and having more precision wouldn't make any pracital sense

# Features:
- Repeat last operation using =
- Predictable operation: only the = sign evaluates the expressions
- Automatic extension with scientific buttons when the window is in landscape mode
- Expressions are more readable, by using superscript, subscript and custom svg radical sign
- Copy/Paste expressions or results
- Trigonometry and radical functions work both before or after inputting an operand
- Intuitive Backspace and AC/C functions
- Native f64 Rust variables to get the highest performance supported by x64 processors while also keeping the code simple:
	- 1-bit sign (+/-)
	- 11-bits exponent (10^308)
	- 52-bits mantissa (fraction) +1 implicit bit which gives 15.95 digits precision (rounded down to a reliable 15 digits)
- Customizable random function
- Customizable button layout

# Will not be suppored:
- Complex numbers and their imaginary units (therefore negative number under sqrt should give an error)
- Programmer's operations: bitshift, binary, hexadecimal calcualtions
- Economic and Graphing calculations (use a spreadsheet for that)
- Date, Currency, Unit conversions (currency would need a data provider and we want to keep this tool to be 100% offline)
- Area, perimeter, volume, surface formulas
- Physics, chemistry formulas/constants

# Video demonstrations:
