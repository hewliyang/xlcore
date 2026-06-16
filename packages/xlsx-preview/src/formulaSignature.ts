export interface FunctionSignature {
  name: string;
  args: string[];
  summary: string;
}

export interface SignatureContext {
  name: string;
  argIndex: number;
}

const SIGNATURES: FunctionSignature[] = [
  {
    name: "ABS",
    args: ["number"],
    summary: "Returns the absolute value of a number.",
  },
  {
    name: "ACOS",
    args: ["number"],
    summary: "Returns the arccosine of a number.",
  },
  {
    name: "ACOSH",
    args: ["number"],
    summary: "Returns the inverse hyperbolic cosine of a number.",
  },
  {
    name: "ACOT",
    args: ["number"],
    summary: "Returns the arccotangent of a number.",
  },
  {
    name: "ACOTH",
    args: ["number"],
    summary: "Returns the inverse hyperbolic cotangent of a number.",
  },
  {
    name: "AND",
    args: ["logical1", "logical2", "..."],
    summary: "Returns TRUE if all of its arguments are TRUE.",
  },
  {
    name: "ARABIC",
    args: ["text"],
    summary: "Converts a Roman numeral to Arabic, as a number.",
  },
  {
    name: "ASIN",
    args: ["number"],
    summary: "Returns the arcsine of a number.",
  },
  {
    name: "ASINH",
    args: ["number"],
    summary: "Returns the inverse hyperbolic sine of a number.",
  },
  {
    name: "ATAN",
    args: ["number"],
    summary: "Returns the arctangent of a number.",
  },
  {
    name: "ATAN2",
    args: ["x_num", "y_num"],
    summary: "Returns the arctangent from x and y coordinates.",
  },
  {
    name: "ATANH",
    args: ["number"],
    summary: "Returns the inverse hyperbolic tangent of a number.",
  },
  {
    name: "AVEDEV",
    args: ["number1", "number2", "..."],
    summary: "Returns the average of the absolute deviations of data points from their mean.",
  },
  {
    name: "AVERAGE",
    args: ["number1", "number2", "..."],
    summary: "Returns the average of its arguments.",
  },
  {
    name: "AVERAGEA",
    args: ["value1", "value2", "..."],
    summary: "Returns the average of its arguments, including numbers, text, and logical values.",
  },
  {
    name: "AVERAGEIF",
    args: ["range", "criteria", "average_range"],
    summary: "Returns the average of all cells in a range that meet a given criteria.",
  },
  {
    name: "AVERAGEIFS",
    args: ["average_range", "criteria_range1", "criteria1", "..."],
    summary: "Returns the average of all cells that meet multiple criteria.",
  },
  {
    name: "BASE",
    args: ["number", "radix", "min_length"],
    summary: "Converts a number into a text representation with the given radix (base).",
  },
  {
    name: "BESSELI",
    args: ["x", "n"],
    summary: "Returns the modified Bessel function In(x).",
  },
  {
    name: "BESSELJ",
    args: ["x", "n"],
    summary: "Returns the Bessel function Jn(x).",
  },
  {
    name: "BESSELK",
    args: ["x", "n"],
    summary: "Returns the modified Bessel function Kn(x).",
  },
  {
    name: "BESSELY",
    args: ["x", "n"],
    summary: "Returns the Bessel function Yn(x).",
  },
  {
    name: "BETA.DIST",
    args: ["x", "alpha", "beta", "cumulative", "A", "B"],
    summary: "Returns the beta cumulative distribution function.",
  },
  {
    name: "BETA.INV",
    args: ["probability", "alpha", "beta", "A", "B"],
    summary: "Returns the inverse of the beta cumulative distribution.",
  },
  {
    name: "BIN2DEC",
    args: ["number"],
    summary: "Converts a binary number to decimal.",
  },
  {
    name: "BIN2HEX",
    args: ["number", "places"],
    summary: "Converts a binary number to hexadecimal.",
  },
  {
    name: "BIN2OCT",
    args: ["number", "places"],
    summary: "Converts a binary number to octal.",
  },
  {
    name: "BINOM.DIST",
    args: ["number_s", "trials", "probability_s", "cumulative"],
    summary: "Returns the individual term binomial distribution probability.",
  },
  {
    name: "BINOM.DIST.RANGE",
    args: ["trials", "probability_s", "number_s", "number_s2"],
    summary: "Returns the probability of a trial result using a binomial distribution.",
  },
  {
    name: "BINOM.INV",
    args: ["trials", "probability_s", "alpha"],
    summary:
      "Returns the smallest value for which the cumulative binomial distribution is greater than or equal to a criterion value.",
  },
  {
    name: "BITAND",
    args: ["number1", "number2"],
    summary: "Returns a bitwise 'AND' of two numbers.",
  },
  {
    name: "BITLSHIFT",
    args: ["number", "shift_amount"],
    summary: "Returns a number shifted left by the specified number of bits.",
  },
  {
    name: "BITOR",
    args: ["number1", "number2"],
    summary: "Returns a bitwise OR of two numbers.",
  },
  {
    name: "BITRSHIFT",
    args: ["number", "shift_amount"],
    summary: "Returns a number shifted right by the specified number of bits.",
  },
  {
    name: "BITXOR",
    args: ["number1", "number2"],
    summary: "Returns a bitwise XOR of two numbers.",
  },
  {
    name: "CEILING",
    args: ["number", "significance"],
    summary:
      "Returns a number rounded up to the nearest integer or to the nearest multiple of significance.",
  },
  {
    name: "CEILING.MATH",
    args: ["number", "significance", "mode"],
    summary:
      "Rounds a number up, to the nearest integer or to the nearest multiple of significance.",
  },
  {
    name: "CEILING.PRECISE",
    args: ["number", "significance"],
    summary:
      "Rounds a number up to the nearest integer or to the nearest multiple of significance.",
  },
  {
    name: "CELL",
    args: ["info_type", "reference"],
    summary: "Returns information about the formatting, location, or contents of a cell.",
  },
  {
    name: "CHISQ.DIST",
    args: ["x", "deg_freedom", "cumulative"],
    summary: "Returns the one-tailed probability of the chi-squared distribution.",
  },
  {
    name: "CHISQ.DIST.RT",
    args: ["x", "deg_freedom"],
    summary: "Returns the right-tailed probability of the chi-squared distribution.",
  },
  {
    name: "CHISQ.INV",
    args: ["probability", "deg_freedom"],
    summary: "Returns the inverse of the left-tailed probability of the chi-squared distribution.",
  },
  {
    name: "CHISQ.INV.RT",
    args: ["probability", "deg_freedom"],
    summary: "Returns the inverse of the right-tailed probability of the chi-squared distribution.",
  },
  {
    name: "CHISQ.TEST",
    args: ["actual_range", "expected_range"],
    summary: "Returns the test for independence.",
  },
  {
    name: "CHOOSE",
    args: ["index_num", "value1", "value2", "..."],
    summary: "Uses an index to return a value from a list of values.",
  },
  {
    name: "COLUMN",
    args: ["reference"],
    summary: "Returns the column number of a reference.",
  },
  {
    name: "COLUMNS",
    args: ["array"],
    summary: "Returns the number of columns in a reference.",
  },
  {
    name: "COMBIN",
    args: ["number", "number_chosen"],
    summary: "Returns the number of combinations for a given number of items.",
  },
  {
    name: "COMBINA",
    args: ["number", "number_chosen"],
    summary: "Returns the number of combinations with repetitions for a given number of items.",
  },
  {
    name: "COMPLEX",
    args: ["real_num", "i_num", "suffix"],
    summary: "Converts real and imaginary coefficients into a complex number.",
  },
  {
    name: "CONCAT",
    args: ["text1", "text2", "..."],
    summary: "Combines the text from multiple ranges and/or strings.",
  },
  {
    name: "CONCATENATE",
    args: ["text1", "text2", "..."],
    summary: "Joins several text strings into one text string.",
  },
  {
    name: "CONFIDENCE.NORM",
    args: ["alpha", "standard_dev", "size"],
    summary: "Returns the confidence interval for a population mean, using a normal distribution.",
  },
  {
    name: "CONFIDENCE.T",
    args: ["alpha", "standard_dev", "size"],
    summary:
      "Returns the confidence interval for a population mean, using a Student's t distribution.",
  },
  {
    name: "CONVERT",
    args: ["number", "from_unit", "to_unit"],
    summary: "Converts a number from one measurement system to another.",
  },
  {
    name: "CORREL",
    args: ["array1", "array2"],
    summary: "Returns the correlation coefficient between two data sets.",
  },
  { name: "COS", args: ["number"], summary: "Returns the cosine of a number." },
  {
    name: "COSH",
    args: ["number"],
    summary: "Returns the hyperbolic cosine of a number.",
  },
  {
    name: "COT",
    args: ["number"],
    summary: "Returns the cotangent of a number.",
  },
  {
    name: "COTH",
    args: ["number"],
    summary: "Returns the hyperbolic cotangent of a number.",
  },
  {
    name: "COUNT",
    args: ["value1", "value2", "..."],
    summary: "Counts how many numbers are in the list of arguments.",
  },
  {
    name: "COUNTA",
    args: ["value1", "value2", "..."],
    summary: "Counts how many values are not empty in a range of cells.",
  },
  {
    name: "COUNTBLANK",
    args: ["range"],
    summary: "Counts the number of empty cells in a range of cells.",
  },
  {
    name: "COUNTIF",
    args: ["range", "criteria"],
    summary: "Counts the number of cells within a range that meet the given criteria.",
  },
  {
    name: "COUNTIFS",
    args: ["criteria_range1", "criteria1", "..."],
    summary: "Counts the number of cells that meet multiple criteria.",
  },
  {
    name: "COVARIANCE.P",
    args: ["array1", "array2"],
    summary:
      "Returns population covariance, the average of the products of deviations for each data point pair.",
  },
  {
    name: "COVARIANCE.S",
    args: ["array1", "array2"],
    summary:
      "Returns the sample covariance, the average of the products of deviations for each data point pair in two data sets.",
  },
  {
    name: "CSC",
    args: ["number"],
    summary: "Returns the cosecant of an angle.",
  },
  {
    name: "CSCH",
    args: ["number"],
    summary: "Returns the hyperbolic cosecant of a number.",
  },
  {
    name: "CUMIPMT",
    args: ["rate", "nper", "pv", "start_period", "end_period", "type"],
    summary: "Returns the cumulative interest paid on a loan between start_period and end_period.",
  },
  {
    name: "CUMPRINC",
    args: ["rate", "nper", "pv", "start_period", "end_period", "type"],
    summary: "Returns the cumulative principal paid on a loan between two periods.",
  },
  {
    name: "DATE",
    args: ["year", "month", "day"],
    summary: "Returns the serial number that represents a particular date.",
  },
  {
    name: "DATEDIF",
    args: ["start_date", "end_date", "unit"],
    summary: "Calculates the number of days, months, or years between two dates.",
  },
  {
    name: "DATEVALUE",
    args: ["date_text"],
    summary: "Converts a date in the form of text to a serial number.",
  },
  {
    name: "DAVERAGE",
    args: ["database", "field", "criteria"],
    summary: "Returns the average of selected database entries.",
  },
  {
    name: "DAY",
    args: ["serial_number"],
    summary: "Returns the day of the month, a number from 1 to 31.",
  },
  {
    name: "DAYS",
    args: ["end_date", "start_date"],
    summary: "Returns the number of days between two dates.",
  },
  {
    name: "DAYS360",
    args: ["start_date", "end_date", "method"],
    summary: "Returns the number of days between two dates based on a 360-day year.",
  },
  {
    name: "DB",
    args: ["cost", "salvage", "life", "period", "month"],
    summary:
      "Returns the depreciation of an asset for a specified period using the fixed-declining balance method.",
  },
  {
    name: "DCOUNT",
    args: ["database", "field", "criteria"],
    summary:
      "Counts the cells containing numbers in the field of records in the database that match the criteria.",
  },
  {
    name: "DCOUNTA",
    args: ["database", "field", "criteria"],
    summary: "Counts the nonblank cells in a column of a list or database.",
  },
  {
    name: "DDB",
    args: ["cost", "salvage", "life", "period", "factor"],
    summary:
      "Returns the depreciation of an asset for a specified period using the double-declining balance method.",
  },
  {
    name: "DEC2BIN",
    args: ["number", "places"],
    summary: "Converts a decimal number to binary.",
  },
  {
    name: "DEC2HEX",
    args: ["number", "places"],
    summary: "Converts a decimal number to hexadecimal.",
  },
  {
    name: "DEC2OCT",
    args: ["number", "places"],
    summary: "Converts a decimal number to octal.",
  },
  {
    name: "DECIMAL",
    args: ["number", "radix"],
    summary: "Converts a text representation of a number in a given base into a decimal number.",
  },
  { name: "DEGREES", args: ["angle"], summary: "Converts radians to degrees." },
  {
    name: "DELTA",
    args: ["number1", "number2"],
    summary: "Tests whether two values are equal.",
  },
  {
    name: "DEVSQ",
    args: ["number1", "number2", "..."],
    summary: "Returns the sum of squares of deviations.",
  },
  {
    name: "DGET",
    args: ["database", "field", "criteria"],
    summary: "Extracts from a database a single record that matches the specified criteria.",
  },
  {
    name: "DMAX",
    args: ["database", "field", "criteria"],
    summary: "Returns the maximum value from selected database entries.",
  },
  {
    name: "DMIN",
    args: ["database", "field", "criteria"],
    summary: "Returns the minimum value from selected database entries.",
  },
  {
    name: "DOLLARDE",
    args: ["fractional_dollar", "fraction"],
    summary: "Converts a dollar price expressed as a fraction into a decimal dollar price.",
  },
  {
    name: "DOLLARFR",
    args: ["decimal_dollar", "fraction"],
    summary:
      "Converts a dollar price expressed as a decimal number into a dollar price expressed as a fraction.",
  },
  {
    name: "DPRODUCT",
    args: ["database", "field", "criteria"],
    summary:
      "Multiplies the values in a column of a list or database that match conditions you specify.",
  },
  {
    name: "DSTDEV",
    args: ["database", "field", "criteria"],
    summary:
      "Estimates the standard deviation of a population based on a sample by using database entries.",
  },
  {
    name: "DSTDEVP",
    args: ["database", "field", "criteria"],
    summary:
      "Calculates the standard deviation based on the entire population of selected database entries.",
  },
  {
    name: "DSUM",
    args: ["database", "field", "criteria"],
    summary:
      "Adds the numbers in the field column of records in the database that match the criteria.",
  },
  {
    name: "DVAR",
    args: ["database", "field", "criteria"],
    summary: "Estimates variance based on a sample from selected database entries.",
  },
  {
    name: "DVARP",
    args: ["number1", "number2", "..."],
    summary: "Returns variance based on the entire population.",
  },
  {
    name: "EDATE",
    args: ["start_date", "months"],
    summary:
      "Returns the serial number of the date that is the indicated number of months before or after the start date.",
  },
  {
    name: "EFFECT",
    args: ["nominal_rate", "npery"],
    summary: "Returns the effective annual interest rate.",
  },
  {
    name: "EOMONTH",
    args: ["start_date", "months"],
    summary:
      "Returns the serial number of the last day of the month before or after a specified number of months.",
  },
  {
    name: "ERF",
    args: ["lower_limit", "upper_limit"],
    summary: "Returns the error function.",
  },
  {
    name: "ERF.PRECISE",
    args: ["lower_limit"],
    summary: "Returns the error function integrated between 0 and a lower limit.",
  },
  {
    name: "ERFC",
    args: ["x"],
    summary: "Returns the complementary ERF function integrated between x and infinity.",
  },
  {
    name: "ERFC.PRECISE",
    args: ["x"],
    summary: "Returns the complementary ERF function integrated between x and infinity.",
  },
  {
    name: "ERROR.TYPE",
    args: ["error_val"],
    summary: "Returns a number corresponding to one of the error values.",
  },
  {
    name: "EVEN",
    args: ["number"],
    summary: "Rounds a number up to the nearest even integer.",
  },
  {
    name: "EXACT",
    args: ["text1", "text2"],
    summary: "Checks whether two text strings are exactly the same.",
  },
  {
    name: "EXP",
    args: ["number"],
    summary: "Returns e raised to the power of a given number.",
  },
  {
    name: "EXPON.DIST",
    args: ["x", "lambda", "cumulative"],
    summary: "Returns the exponential distribution.",
  },
  {
    name: "F.DIST",
    args: ["x", "deg_freedom1", "deg_freedom2", "cumulative"],
    summary: "Returns the F probability distribution.",
  },
  {
    name: "F.DIST.RT",
    args: ["x", "deg_freedom1", "deg_freedom2"],
    summary: "Returns the right-tailed F probability distribution.",
  },
  {
    name: "F.INV",
    args: ["probability", "deg_freedom1", "deg_freedom2"],
    summary: "Returns the inverse of the F probability distribution.",
  },
  {
    name: "F.INV.RT",
    args: ["probability", "deg_freedom1", "deg_freedom2"],
    summary: "Returns the inverse of the right-tailed F probability distribution.",
  },
  {
    name: "F.TEST",
    args: ["array1", "array2"],
    summary: "Returns the result of an F-test.",
  },
  {
    name: "FACT",
    args: ["number"],
    summary: "Returns the factorial of a number.",
  },
  {
    name: "FACTDOUBLE",
    args: ["number"],
    summary: "Returns the double factorial of a number.",
  },
  { name: "FALSE", args: [], summary: "Returns the logical value FALSE." },
  {
    name: "FIND",
    args: ["find_text", "within_text", "start_num"],
    summary: "Finds one text value within another (case-sensitive).",
  },
  {
    name: "FISHER",
    args: ["x"],
    summary: "Returns the Fisher transformation.",
  },
  {
    name: "FISHERINV",
    args: ["y"],
    summary: "Returns the inverse of the Fisher transformation.",
  },
  {
    name: "FLOOR",
    args: ["number", "significance"],
    summary: "Rounds a number down, toward zero, to the nearest multiple of significance.",
  },
  {
    name: "FLOOR.MATH",
    args: ["number", "significance", "mode"],
    summary:
      "Rounds a number down to the nearest integer or to the nearest multiple of significance.",
  },
  {
    name: "FLOOR.PRECISE",
    args: ["number", "significance"],
    summary:
      "Rounds a number down to the nearest integer or to the nearest multiple of significance.",
  },
  {
    name: "FORMULATEXT",
    args: ["reference"],
    summary: "Returns the formula as a text string.",
  },
  {
    name: "FV",
    args: ["rate", "nper", "pmt", "pv", "type"],
    summary: "Returns the future value of an investment.",
  },
  {
    name: "GAMMA",
    args: ["number"],
    summary: "Returns the Gamma function value.",
  },
  {
    name: "GAMMA.DIST",
    args: ["x", "alpha", "beta", "cumulative"],
    summary: "Returns the gamma distribution.",
  },
  {
    name: "GAMMA.INV",
    args: ["probability", "alpha", "beta"],
    summary: "Returns the inverse of the gamma cumulative distribution.",
  },
  {
    name: "GAMMALN",
    args: ["x"],
    summary: "Returns the natural logarithm of the gamma function.",
  },
  {
    name: "GAMMALN.PRECISE",
    args: ["x"],
    summary: "Returns the natural logarithm of the gamma function, Γ(x).",
  },
  {
    name: "GAUSS",
    args: ["z"],
    summary: "Returns 0.5 less than the standard normal cumulative distribution.",
  },
  {
    name: "GCD",
    args: ["number1", "number2", "..."],
    summary: "Returns the greatest common divisor.",
  },
  {
    name: "GEOMEAN",
    args: ["number1", "number2", "..."],
    summary: "Returns the geometric mean of an array or range of positive data.",
  },
  {
    name: "GESTEP",
    args: ["number", "step"],
    summary: "Tests whether a number is greater than a threshold value.",
  },
  {
    name: "HARMEAN",
    args: ["number1", "number2", "..."],
    summary: "Returns the harmonic mean of a data set.",
  },
  {
    name: "HEX2BIN",
    args: ["number", "places"],
    summary: "Converts a hexadecimal number to binary.",
  },
  {
    name: "HEX2DEC",
    args: ["number"],
    summary: "Converts a hexadecimal number to decimal.",
  },
  {
    name: "HEX2OCT",
    args: ["number", "places"],
    summary: "Converts a hexadecimal number to octal.",
  },
  {
    name: "HLOOKUP",
    args: ["lookup_value", "table_array", "row_index_num", "range_lookup"],
    summary:
      "Looks up a value in the top row of a table and returns a value in the same column from a row you specify.",
  },
  {
    name: "HOUR",
    args: ["serial_number"],
    summary: "Returns the hour as a number from 0 (12:00 A.M.) to 23 (11:00 P.M.).",
  },
  {
    name: "HYPGEOM.DIST",
    args: ["sample_s", "number_sample", "population_s", "number_pop", "cumulative"],
    summary: "Returns the hypergeometric distribution.",
  },
  {
    name: "IF",
    args: ["logical_test", "value_if_true", "value_if_false"],
    summary: "Specifies a logical test to perform.",
  },
  {
    name: "IFERROR",
    args: ["value", "value_if_error"],
    summary:
      "Returns a value you specify if a formula evaluates to an error; otherwise, returns the result of the formula.",
  },
  {
    name: "IFNA",
    args: ["value", "value_if_na"],
    summary:
      "Returns the value you specify if the expression resolves to #N/A; otherwise returns the expression result.",
  },
  {
    name: "IFS",
    args: ["logical_test1", "value_if_true1", "logical_test2", "value_if_true2", "..."],
    summary:
      "Checks whether one or more conditions are met and returns a value corresponding to the first TRUE condition.",
  },
  {
    name: "IMABS",
    args: ["inumber"],
    summary: "Returns the absolute value (modulus) of a complex number.",
  },
  {
    name: "IMAGINARY",
    args: ["inumber"],
    summary: "Returns the imaginary coefficient of a complex number.",
  },
  {
    name: "IMARGUMENT",
    args: ["inumber"],
    summary: "Returns the argument theta, an angle expressed in radians.",
  },
  {
    name: "IMCONJUGATE",
    args: ["inumber"],
    summary: "Returns the complex conjugate of a complex number.",
  },
  {
    name: "IMCOS",
    args: ["inumber"],
    summary: "Returns the cosine of a complex number.",
  },
  {
    name: "IMCOSH",
    args: ["inumber"],
    summary: "Returns the hyperbolic cosine of a complex number.",
  },
  {
    name: "IMCOT",
    args: ["inumber"],
    summary: "Returns the cotangent of a complex number.",
  },
  {
    name: "IMCSC",
    args: ["inumber"],
    summary: "Returns the cosecant of a complex number.",
  },
  {
    name: "IMCSCH",
    args: ["inumber"],
    summary: "Returns the hyperbolic cosecant of a complex number.",
  },
  {
    name: "IMDIV",
    args: ["inumber1", "inumber2"],
    summary: "Returns the quotient of two complex numbers.",
  },
  {
    name: "IMEXP",
    args: ["inumber"],
    summary: "Returns the exponential of a complex number.",
  },
  {
    name: "IMLN",
    args: ["inumber"],
    summary: "Returns the natural logarithm of a complex number.",
  },
  {
    name: "IMLOG10",
    args: ["inumber"],
    summary: "Returns the base-10 logarithm of a complex number.",
  },
  {
    name: "IMLOG2",
    args: ["inumber"],
    summary: "Returns the base-2 logarithm of a complex number.",
  },
  {
    name: "IMPOWER",
    args: ["inumber", "number"],
    summary: "Returns a complex number raised to an integer power.",
  },
  {
    name: "IMPRODUCT",
    args: ["inumber1", "inumber2", "..."],
    summary: "Returns the product of up to 255 complex numbers.",
  },
  {
    name: "IMREAL",
    args: ["inumber"],
    summary: "Returns the real coefficient of a complex number.",
  },
  {
    name: "IMSEC",
    args: ["inumber"],
    summary: "Returns the secant of a complex number.",
  },
  {
    name: "IMSECH",
    args: ["inumber"],
    summary: "Returns the hyperbolic secant of a complex number.",
  },
  {
    name: "IMSIN",
    args: ["inumber"],
    summary: "Returns the sine of a complex number.",
  },
  {
    name: "IMSINH",
    args: ["inumber"],
    summary: "Returns the hyperbolic sine of a complex number.",
  },
  {
    name: "IMSQRT",
    args: ["inumber"],
    summary: "Returns the square root of a complex number.",
  },
  {
    name: "IMSUB",
    args: ["inumber1", "inumber2"],
    summary: "Returns the difference of two complex numbers.",
  },
  {
    name: "IMSUM",
    args: ["inumber1", "inumber2", "..."],
    summary: "Returns the sum of complex numbers.",
  },
  {
    name: "IMTAN",
    args: ["inumber"],
    summary: "Returns the tangent of a complex number.",
  },
  {
    name: "INDEX",
    args: ["array", "row_num", "column_num", "area_num"],
    summary: "Returns a value or the reference to a value from within a range or table.",
  },
  {
    name: "INDIRECT",
    args: ["ref_text", "a1"],
    summary: "Returns a reference indicated by a text value.",
  },
  {
    name: "INFO",
    args: ["type_text"],
    summary: "Returns information about the current operating environment.",
  },
  {
    name: "INT",
    args: ["number"],
    summary: "Rounds a number down to the nearest integer.",
  },
  {
    name: "INTERCEPT",
    args: ["known_y's", "known_x's"],
    summary: "Returns the y-intercept of the linear regression line.",
  },
  {
    name: "IPMT",
    args: ["rate", "per", "nper", "pv", "fv", "type"],
    summary: "Returns the interest payment for an investment for a given period.",
  },
  {
    name: "IRR",
    args: ["values", "guess"],
    summary: "Returns the internal rate of return for a series of cash flows.",
  },
  {
    name: "ISBLANK",
    args: ["value"],
    summary: "Returns TRUE if the value is blank.",
  },
  {
    name: "ISERR",
    args: ["value"],
    summary: "Checks whether a value is an error other than #N/A.",
  },
  {
    name: "ISERROR",
    args: ["value"],
    summary: "Returns TRUE if the value is any error value except #N/A.",
  },
  {
    name: "ISEVEN",
    args: ["number"],
    summary: "Returns TRUE if the number is even.",
  },
  {
    name: "ISFORMULA",
    args: ["reference"],
    summary:
      "Checks whether a reference is to a cell containing a formula, and returns TRUE or FALSE.",
  },
  {
    name: "ISLOGICAL",
    args: ["value"],
    summary: "Returns TRUE if the value is a logical value.",
  },
  {
    name: "ISNA",
    args: ["value"],
    summary: "Returns TRUE if the value is the #N/A error value.",
  },
  {
    name: "ISNONTEXT",
    args: ["value"],
    summary: "Checks whether a value is not text.",
  },
  {
    name: "ISNUMBER",
    args: ["value"],
    summary: "Returns TRUE if the value is a number.",
  },
  {
    name: "ISO.CEILING",
    args: ["number", "significance"],
    summary:
      "Returns a number that is rounded up to the nearest integer or to the nearest multiple of significance.",
  },
  {
    name: "ISODD",
    args: ["number"],
    summary: "Returns TRUE if the number is odd.",
  },
  {
    name: "ISOWEEKNUM",
    args: ["date"],
    summary: "Returns the number of the ISO week number of the year for a given date.",
  },
  {
    name: "ISPMT",
    args: ["rate", "per", "nper", "pv"],
    summary: "Returns the interest paid during a specific period of an investment.",
  },
  {
    name: "ISREF",
    args: ["value"],
    summary: "Checks whether a value is a reference.",
  },
  {
    name: "ISTEXT",
    args: ["value"],
    summary: "Returns TRUE if the value is text.",
  },
  {
    name: "KURT",
    args: ["number1", "number2", "..."],
    summary: "Returns the kurtosis of a data set.",
  },
  {
    name: "LARGE",
    args: ["array", "k"],
    summary: "Returns the k-th largest value in a data set.",
  },
  {
    name: "LCM",
    args: ["number1", "number2", "..."],
    summary: "Returns the least common multiple.",
  },
  {
    name: "LEFT",
    args: ["text", "num_chars"],
    summary: "Returns the leftmost characters from a text value.",
  },
  {
    name: "LEN",
    args: ["text"],
    summary: "Returns the number of characters in a text string.",
  },
  {
    name: "LN",
    args: ["number"],
    summary: "Returns the natural logarithm of a number.",
  },
  {
    name: "LOG",
    args: ["number", "base"],
    summary: "Returns the logarithm of a number to the base you specify.",
  },
  {
    name: "LOG10",
    args: ["number"],
    summary: "Returns the base-10 logarithm of a number.",
  },
  {
    name: "LOGNORM.DIST",
    args: ["x", "mean", "standard_dev", "cumulative"],
    summary: "Returns the lognormal cumulative distribution.",
  },
  {
    name: "LOGNORM.INV",
    args: ["probability", "mean", "standard_dev"],
    summary: "Returns the inverse of the lognormal distribution.",
  },
  {
    name: "LOOKUP",
    args: ["lookup_value", "lookup_vector", "result_vector"],
    summary: "Looks up a value from a one-row or one-column range or from an array.",
  },
  { name: "LOWER", args: ["text"], summary: "Converts text to lowercase." },
  {
    name: "MATCH",
    args: ["lookup_value", "lookup_array", "match_type"],
    summary:
      "Returns the relative position of an item in an array that matches a specified value in a specified order.",
  },
  {
    name: "MAX",
    args: ["number1", "number2", "..."],
    summary: "Returns the maximum value in a list of arguments.",
  },
  {
    name: "MAXA",
    args: ["number1", "number2", "..."],
    summary:
      "Returns the largest value in a list of arguments, including numbers, text, and logical values.",
  },
  {
    name: "MAXIFS",
    args: ["max_range", "criteria_range1", "criteria1", "..."],
    summary:
      "Returns the maximum value among cells specified by a given set of conditions or criteria.",
  },
  {
    name: "MEDIAN",
    args: ["number1", "number2", "..."],
    summary: "Returns the median of the given numbers.",
  },
  {
    name: "MID",
    args: ["text", "start_num", "num_chars"],
    summary:
      "Returns a specific number of characters from a text string starting at the position you specify.",
  },
  {
    name: "MIN",
    args: ["number1", "number2", "..."],
    summary: "Returns the minimum value in a list of arguments.",
  },
  {
    name: "MINA",
    args: ["value1", "value2", "..."],
    summary:
      "Returns the minimum value in a list of arguments, including numbers, text, and logical values.",
  },
  {
    name: "MINIFS",
    args: ["min_range", "criteria_range1", "criteria1", "..."],
    summary: "Returns the minimum value among cells specified by a given set of criteria.",
  },
  {
    name: "MINUTE",
    args: ["serial_number"],
    summary: "Converts a serial number to a minute.",
  },
  {
    name: "MIRR",
    args: ["values", "finance_rate", "reinvest_rate"],
    summary: "Returns the internal rate of return for a series of periodic cash flows.",
  },
  {
    name: "MOD",
    args: ["number", "divisor"],
    summary: "Returns the remainder after a number is divided by a divisor.",
  },
  {
    name: "MONTH",
    args: ["serial_number"],
    summary: "Returns the month, a number from 1 to 12.",
  },
  {
    name: "MROUND",
    args: ["number", "multiple"],
    summary: "Returns a number rounded to the desired multiple.",
  },
  {
    name: "N",
    args: ["value"],
    summary: "Returns a value converted to a number.",
  },
  { name: "NA", args: [], summary: "Returns the error value #N/A." },
  {
    name: "NEGBINOM.DIST",
    args: ["number_f", "number_s", "probability_s", "cumulative"],
    summary: "Returns the negative binomial distribution.",
  },
  {
    name: "NETWORKDAYS",
    args: ["start_date", "end_date", "holidays"],
    summary: "Returns the number of whole workdays between two dates.",
  },
  {
    name: "NETWORKDAYS.INTL",
    args: ["start_date", "end_date", "weekend", "holidays"],
    summary:
      "Returns the number of whole workdays between two dates using parameters to indicate weekend days.",
  },
  {
    name: "NOMINAL",
    args: ["effect_rate", "npery"],
    summary: "Returns the annual nominal interest rate.",
  },
  {
    name: "NORM.DIST",
    args: ["x", "mean", "standard_dev", "cumulative"],
    summary: "Returns the normal cumulative distribution.",
  },
  {
    name: "NORM.INV",
    args: ["probability", "mean", "standard_dev"],
    summary: "Returns the inverse of the standard normal cumulative distribution.",
  },
  {
    name: "NORM.S.DIST",
    args: ["z", "cumulative"],
    summary: "Returns the standard normal cumulative distribution.",
  },
  {
    name: "NORM.S.INV",
    args: ["probability"],
    summary: "Returns the inverse of the standard normal cumulative distribution.",
  },
  {
    name: "NOT",
    args: ["logical"],
    summary: "Reverses the logic of its argument.",
  },
  {
    name: "NOW",
    args: [],
    summary: "Returns the serial number of the current date and time.",
  },
  {
    name: "NPER",
    args: ["rate", "pmt", "pv", "fv", "type"],
    summary:
      "Returns the number of periods for an investment based on periodic, constant payments and a constant interest rate.",
  },
  {
    name: "NPV",
    args: ["rate", "value1", "value2", "..."],
    summary:
      "Returns the net present value of an investment based on a series of periodic cash flows and a discount rate.",
  },
  {
    name: "OCT2BIN",
    args: ["number", "places"],
    summary: "Converts an octal number to binary.",
  },
  {
    name: "OCT2DEC",
    args: ["number"],
    summary: "Converts an octal number to decimal.",
  },
  {
    name: "OCT2HEX",
    args: ["number", "places"],
    summary: "Converts an octal number to hexadecimal.",
  },
  {
    name: "ODD",
    args: ["number"],
    summary: "Rounds a number up to the nearest odd integer.",
  },
  {
    name: "OFFSET",
    args: ["reference", "rows", "cols", "height", "width"],
    summary: "Returns a reference offset from a given reference.",
  },
  {
    name: "OR",
    args: ["logical1", "logical2", "..."],
    summary: "Returns TRUE if any of the arguments are TRUE.",
  },
  {
    name: "PDURATION",
    args: ["rate", "pv", "fv"],
    summary: "Returns the number of periods required for an investment to reach a specified value.",
  },
  {
    name: "PEARSON",
    args: ["array1", "array2"],
    summary: "Returns the Pearson product moment correlation coefficient.",
  },
  {
    name: "PHI",
    args: ["x"],
    summary: "Returns the value of the standard normal density function.",
  },
  { name: "PI", args: [], summary: "Returns the value of pi." },
  {
    name: "PMT",
    args: ["rate", "nper", "pv", "fv", "type"],
    summary: "Returns the periodic payment for an annuity.",
  },
  {
    name: "POISSON.DIST",
    args: ["x", "mean", "cumulative"],
    summary: "Returns the Poisson distribution.",
  },
  {
    name: "POWER",
    args: ["number", "power"],
    summary: "Returns the result of a number raised to a power.",
  },
  {
    name: "PPMT",
    args: ["rate", "per", "nper", "pv", "fv", "type"],
    summary:
      "Returns the payment on the principal for a given period for an investment based on periodic, constant payments and a constant interest rate.",
  },
  {
    name: "PRODUCT",
    args: ["number1", "number2", "..."],
    summary: "Multiplies all the numbers given as arguments.",
  },
  {
    name: "PV",
    args: ["rate", "nper", "pmt", "fv", "type"],
    summary: "Returns the present value of an investment.",
  },
  {
    name: "QUOTIENT",
    args: ["numerator", "denominator"],
    summary: "Returns the integer portion of a division.",
  },
  { name: "RADIANS", args: ["angle"], summary: "Converts degrees to radians." },
  {
    name: "RAND",
    args: [],
    summary: "Returns a random number greater than or equal to 0 and less than 1.",
  },
  {
    name: "RANDBETWEEN",
    args: ["bottom", "top"],
    summary: "Returns a random integer between the numbers you specify.",
  },
  {
    name: "RANK.AVG",
    args: ["number", "ref", "order"],
    summary:
      "Returns the rank of a number in a list of numbers; if more than one value has the same rank, returns the average rank.",
  },
  {
    name: "RANK.EQ",
    args: ["number", "ref", "order"],
    summary: "Returns the rank of a number in a list of numbers.",
  },
  {
    name: "RATE",
    args: ["nper", "pmt", "pv", "fv", "type", "guess"],
    summary: "Returns the interest rate per period of an annuity.",
  },
  {
    name: "REPT",
    args: ["text", "number_times"],
    summary: "Repeats text a given number of times.",
  },
  {
    name: "RIGHT",
    args: ["text", "num_chars"],
    summary: "Returns the rightmost characters from a text value.",
  },
  {
    name: "ROMAN",
    args: ["number", "form"],
    summary: "Converts an Arabic numeral to Roman, as text.",
  },
  {
    name: "ROUND",
    args: ["number", "num_digits"],
    summary: "Rounds a number to a specified number of digits.",
  },
  {
    name: "ROUNDDOWN",
    args: ["number", "num_digits"],
    summary: "Rounds a number down, toward zero.",
  },
  {
    name: "ROUNDUP",
    args: ["number", "num_digits"],
    summary: "Rounds a number up, away from zero.",
  },
  {
    name: "ROW",
    args: ["reference"],
    summary: "Returns the row number of a reference.",
  },
  {
    name: "ROWS",
    args: ["array"],
    summary: "Returns the number of rows in a reference or array.",
  },
  {
    name: "RRI",
    args: ["nper", "pv", "fv"],
    summary: "Returns an equivalent interest rate for the growth of an investment.",
  },
  {
    name: "RSQ",
    args: ["known_y's", "known_x's"],
    summary: "Returns the square of the Pearson product moment correlation coefficient.",
  },
  {
    name: "SEARCH",
    args: ["find_text", "within_text", "start_num"],
    summary: "Finds one text string within another (not case-sensitive).",
  },
  { name: "SEC", args: ["number"], summary: "Returns the secant of an angle." },
  {
    name: "SECH",
    args: ["number"],
    summary: "Returns the hyperbolic secant of a number.",
  },
  {
    name: "SECOND",
    args: ["serial_number"],
    summary: "Converts a serial number to a second.",
  },
  {
    name: "SHEET",
    args: ["value"],
    summary: "Returns the sheet number of the referenced sheet.",
  },
  {
    name: "SHEETS",
    args: ["reference"],
    summary: "Returns the number of sheets in a reference.",
  },
  { name: "SIGN", args: ["number"], summary: "Returns the sign of a number." },
  {
    name: "SIN",
    args: ["number"],
    summary: "Returns the sine of the given angle.",
  },
  {
    name: "SINH",
    args: ["number"],
    summary: "Returns the hyperbolic sine of a number.",
  },
  {
    name: "SKEW",
    args: ["number1", "number2", "..."],
    summary: "Returns the skewness of a distribution.",
  },
  {
    name: "SKEW.P",
    args: ["number1", "number2", "..."],
    summary: "Returns the skewness of a distribution based on a population.",
  },
  {
    name: "SLN",
    args: ["cost", "salvage", "life"],
    summary: "Returns the straight-line depreciation of an asset for one period.",
  },
  {
    name: "SLOPE",
    args: ["known_y's", "known_x's"],
    summary: "Returns the slope of the linear regression line.",
  },
  {
    name: "SMALL",
    args: ["array", "k"],
    summary: "Returns the k-th smallest value in a data set.",
  },
  {
    name: "SQRT",
    args: ["number"],
    summary: "Returns the positive square root of a number.",
  },
  {
    name: "SQRTPI",
    args: ["number"],
    summary: "Returns the square root of (number * pi).",
  },
  {
    name: "STANDARDIZE",
    args: ["x", "mean", "standard_dev"],
    summary:
      "Returns a normalized value from a distribution characterized by mean and standard deviation.",
  },
  {
    name: "STDEV.P",
    args: ["number1", "number2", "..."],
    summary: "Calculates standard deviation based on the entire population.",
  },
  {
    name: "STDEV.S",
    args: ["number1", "number2", "..."],
    summary: "Estimates standard deviation based on a sample.",
  },
  {
    name: "STDEVA",
    args: ["number1", "number2", "..."],
    summary:
      "Estimates standard deviation based on a sample, including numbers, text, and logical values.",
  },
  {
    name: "STDEVPA",
    args: ["number1", "number2", "..."],
    summary:
      "Calculates standard deviation based on the entire population, including numbers, text, and logical values.",
  },
  {
    name: "STEYX",
    args: ["known_y's", "known_x's"],
    summary: "Returns the standard error of the predicted y-value for each x in the regression.",
  },
  {
    name: "SUBSTITUTE",
    args: ["text", "old_text", "new_text", "instance_num"],
    summary: "Substitutes new text for old text in a text string.",
  },
  {
    name: "SUBTOTAL",
    args: ["function_num", "ref1", "ref2", "..."],
    summary: "Returns a subtotal in a list or database.",
  },
  {
    name: "SUM",
    args: ["number1", "number2", "..."],
    summary: "Adds all the numbers in a range of cells.",
  },
  {
    name: "SUMIF",
    args: ["range", "criteria", "sum_range"],
    summary: "Adds the cells specified by a given criteria.",
  },
  {
    name: "SUMIFS",
    args: ["sum_range", "criteria_range1", "criteria1", "..."],
    summary: "Adds the cells specified by a given set of conditions or criteria.",
  },
  {
    name: "SUMSQ",
    args: ["number1", "number2", "..."],
    summary: "Returns the sum of the squares of the arguments.",
  },
  {
    name: "SUMX2MY2",
    args: ["array_x", "array_y"],
    summary: "Returns the sum of the difference of squares of corresponding values in two arrays.",
  },
  {
    name: "SUMX2PY2",
    args: ["array_x", "array_y"],
    summary: "Returns the sum of the sum of squares of corresponding values in two arrays.",
  },
  {
    name: "SUMXMY2",
    args: ["array_x", "array_y"],
    summary: "Returns the sum of squares of differences of corresponding values in two arrays.",
  },
  {
    name: "SWITCH",
    args: ["expression", "value1", "result1", "..."],
    summary:
      "Evaluates an expression against a list of values and returns the result corresponding to the first matching value.",
  },
  {
    name: "SYD",
    args: ["cost", "salvage", "life", "per"],
    summary: "Returns the sum-of-years' digits depreciation of an asset for a specified period.",
  },
  { name: "T", args: ["value"], summary: "Converts its arguments to text." },
  {
    name: "T.DIST",
    args: ["x", "deg_freedom", "cumulative"],
    summary: "Returns the left-tailed Student's t-distribution.",
  },
  {
    name: "T.DIST.2T",
    args: ["x", "deg_freedom"],
    summary: "Returns the two-tailed Student's t-distribution.",
  },
  {
    name: "T.DIST.RT",
    args: ["x", "deg_freedom"],
    summary: "Returns the right-tailed Student's t-distribution.",
  },
  {
    name: "T.INV",
    args: ["probability", "deg_freedom"],
    summary: "Returns the left-tailed inverse of the Student's t-distribution.",
  },
  {
    name: "T.INV.2T",
    args: ["probability", "deg_freedom"],
    summary: "Returns the inverse of the Student's t-distribution (two-tailed).",
  },
  {
    name: "T.TEST",
    args: ["array1", "array2", "tails", "type"],
    summary: "Returns the probability associated with Student's t-test.",
  },
  {
    name: "TAN",
    args: ["number"],
    summary: "Returns the tangent of a number.",
  },
  {
    name: "TANH",
    args: ["number"],
    summary: "Returns the hyperbolic tangent of a number.",
  },
  {
    name: "TBILLEQ",
    args: ["settlement", "maturity", "discount"],
    summary: "Returns the bond-equivalent yield for a Treasury bill.",
  },
  {
    name: "TBILLPRICE",
    args: ["settlement", "maturity", "discount"],
    summary: "Returns the price per $100 face value for a Treasury bill.",
  },
  {
    name: "TBILLYIELD",
    args: ["settlement", "maturity", "discount"],
    summary: "Returns the yield for a Treasury bill.",
  },
  {
    name: "TEXT",
    args: ["value", "format_text"],
    summary: "Formats a number and converts it to text.",
  },
  {
    name: "TEXTAFTER",
    args: ["text", "delimiter", "instance_num", "match_mode", "match_end", "if_not_found"],
    summary: "Returns text that occurs after a given character or string.",
  },
  {
    name: "TEXTBEFORE",
    args: ["text", "delimiter", "instance_num", "match_mode", "match_end", "if_not_found"],
    summary: "Returns text that occurs before a given character or string.",
  },
  {
    name: "TEXTJOIN",
    args: ["delimiter", "ignore_empty", "text1", "..."],
    summary: "Concatenates a range or list of text strings using a delimiter.",
  },
  {
    name: "TIME",
    args: ["hour", "minute", "second"],
    summary: "Returns the serial number of a particular time.",
  },
  {
    name: "TIMEVALUE",
    args: ["time_text"],
    summary: "Converts a time in the form of text to a serial number.",
  },
  {
    name: "TODAY",
    args: [],
    summary: "Returns the serial number of today's date.",
  },
  {
    name: "TRIM",
    args: ["text"],
    summary: "Removes all spaces from text except for single spaces between words.",
  },
  { name: "TRUE", args: [], summary: "Returns the logical value TRUE." },
  {
    name: "TRUNC",
    args: ["number", "num_digits"],
    summary: "Truncates a number to an integer.",
  },
  {
    name: "TYPE",
    args: ["value"],
    summary: "Returns an integer representing the data type of a value.",
  },
  {
    name: "UNICODE",
    args: ["text"],
    summary: "Returns the number (code point) corresponding to the first character of the text.",
  },
  { name: "UPPER", args: ["text"], summary: "Converts text to uppercase." },
  {
    name: "VALUE",
    args: ["text"],
    summary: "Converts a text string that represents a number to a number.",
  },
  {
    name: "VALUETOTEXT",
    args: ["value", "format"],
    summary: "Returns text from any specified value.",
  },
  {
    name: "VAR.P",
    args: ["number1", "number2", "..."],
    summary: "Calculates variance based on the entire population.",
  },
  {
    name: "VAR.S",
    args: ["number1", "number2", "..."],
    summary: "Estimates variance based on a sample.",
  },
  {
    name: "VARA",
    args: ["number1", "number2", "..."],
    summary: "Estimates variance based on a sample, including numbers, text, and logical values.",
  },
  {
    name: "VARPA",
    args: ["number1", "number2", "..."],
    summary: "Calculates variance based on the entire population.",
  },
  {
    name: "VLOOKUP",
    args: ["lookup_value", "table_array", "col_index_num", "range_lookup"],
    summary:
      "Looks in the first column of a range for a value and returns a value in the same row from a column you specify.",
  },
  {
    name: "WEEKDAY",
    args: ["serial_number", "return_type"],
    summary: "Returns the day of the week corresponding to a date.",
  },
  {
    name: "WEEKNUM",
    args: ["serial_number", "return_type"],
    summary: "Returns the week number in a year.",
  },
  {
    name: "WEIBULL.DIST",
    args: ["x", "alpha", "beta", "cumulative"],
    summary: "Returns the Weibull distribution.",
  },
  {
    name: "WORKDAY",
    args: ["start_date", "days", "holidays"],
    summary:
      "Returns the serial number of the date before or after a specified number of workdays.",
  },
  {
    name: "WORKDAY.INTL",
    args: ["start_date", "days", "weekend", "holidays"],
    summary:
      "Returns the serial number of the date before or after a specified number of workdays with custom weekend parameters.",
  },
  {
    name: "XIRR",
    args: ["values", "dates", "guess"],
    summary:
      "Returns the internal rate of return for a schedule of cash flows that is not necessarily periodic.",
  },
  {
    name: "XLOOKUP",
    args: [
      "lookup_value",
      "lookup_array",
      "return_array",
      "if_not_found",
      "match_mode",
      "search_mode",
    ],
    summary:
      "Searches a range or an array, and returns an item corresponding to the first match it finds.",
  },
  {
    name: "XNPV",
    args: ["rate", "values", "dates"],
    summary:
      "Returns the net present value for a schedule of cash flows that is not necessarily periodic.",
  },
  {
    name: "XOR",
    args: ["logical1", "logical2", "..."],
    summary: "Returns a logical exclusive OR of all arguments.",
  },
  {
    name: "YEAR",
    args: ["serial_number"],
    summary: "Returns the year corresponding to a date.",
  },
  {
    name: "YEARFRAC",
    args: ["start_date", "end_date", "basis"],
    summary:
      "Returns the year fraction representing the number of whole days between start_date and end_date.",
  },
  {
    name: "Z.TEST",
    args: ["array", "x", "sigma"],
    summary: "Returns the one-tailed P-value of a z-test.",
  },
];

const SIGNATURE_BY_NAME = new Map<string, FunctionSignature>(SIGNATURES.map((s) => [s.name, s]));

function isNameChar(ch: string): boolean {
  return /[A-Za-z0-9._]/.test(ch);
}

function nameBefore(text: string, openIndex: number): string | null {
  let start = openIndex - 1;
  while (start >= 1 && isNameChar(text[start]!)) start--;
  start++;
  if (start >= openIndex) return null;
  const name = text.slice(start, openIndex);
  if (!/[A-Za-z]/.test(name[0]!)) return null;
  return name;
}

export function signatureAt(text: string, caretIndex: number): SignatureContext | null {
  if (!text.startsWith("=")) return null;
  if (caretIndex < 0 || caretIndex > text.length) return null;

  const stack: Array<{ name: string; callDepth: number; argIndex: number }> = [];
  let parenDepth = 0;
  let inString = false;
  let inQuote = false;

  for (let i = 1; i < caretIndex; i++) {
    const ch = text[i];
    if (inString) {
      if (ch === '"') {
        if (text[i + 1] === '"') i++;
        else inString = false;
      }
      continue;
    }
    if (inQuote) {
      if (ch === "'") {
        if (text[i + 1] === "'") i++;
        else inQuote = false;
      }
      continue;
    }
    if (ch === '"') {
      inString = true;
      continue;
    }
    if (ch === "'") {
      inQuote = true;
      continue;
    }
    if (ch === "(") {
      parenDepth++;
      const name = nameBefore(text, i);
      if (name)
        stack.push({
          name: name.toUpperCase(),
          callDepth: parenDepth,
          argIndex: 0,
        });
    } else if (ch === ")") {
      const top = stack[stack.length - 1];
      if (top && top.callDepth === parenDepth) stack.pop();
      parenDepth--;
    } else if (ch === ",") {
      const top = stack[stack.length - 1];
      if (top && top.callDepth === parenDepth) top.argIndex++;
    }
  }

  const top = stack[stack.length - 1];
  if (!top) return null;
  return { name: top.name, argIndex: top.argIndex };
}

export function lookupSignature(name: string): FunctionSignature | null {
  return SIGNATURE_BY_NAME.get(name.toUpperCase()) ?? null;
}
