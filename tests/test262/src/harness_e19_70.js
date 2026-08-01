
// E19.70: regExpUtils.js — Unicode property escape helpers.
function buildString(args) {
  var loneCodePoints = args.loneCodePoints;
  var ranges = args.ranges;
  var CHUNK_SIZE = 10000;
  var result = String.fromCodePoint.apply(null, loneCodePoints);
  var i = 0;
  while (i < ranges.length) {
    var range = ranges[i];
    var start = range[0];
    var end = range[1];
    var codePoints = [];
    var length = 0;
    var codePoint = start;
    while (codePoint <= end) {
      codePoints[length] = codePoint;
      length = length + 1;
      if (length === CHUNK_SIZE) {
        result = result + String.fromCodePoint.apply(null, codePoints);
        codePoints = [];
        length = 0;
      }
      codePoint = codePoint + 1;
    }
    if (length > 0) {
      result = result + String.fromCodePoint.apply(null, codePoints);
    }
    i = i + 1;
  }
  return result;
}

function printCodePoint(codePoint) {
  var hex = codePoint.toString(16).toUpperCase();
  while (hex.length < 6) {
    hex = "0" + hex;
  }
  return "U+" + hex;
}

function printStringCodePoints(string) {
  var buf = [];
  var iter = string[Symbol.iterator]();
  var step = iter.next();
  while (!step.done) {
    var symbol = step.value;
    buf.push(printCodePoint(symbol.codePointAt(0)));
    step = iter.next();
  }
  return buf.join(" ");
}

function testPropertyEscapes(regExp, string, expression) {
  if (!regExp.test(string)) {
    var iter = string[Symbol.iterator]();
    var step = iter.next();
    while (!step.done) {
      var symbol = step.value;
      var formatted = printCodePoint(symbol.codePointAt(0));
      assert(
        regExp.test(symbol),
        "`" + expression + "` should match " + formatted + " (`" + symbol + "`)"
      );
      step = iter.next();
    }
  }
}

function testPropertyOfStrings(args) {
  var regExp = args.regExp;
  var expression = args.expression;
  var matchStrings = args.matchStrings;
  var nonMatchStrings = args.nonMatchStrings;
  var allStrings = matchStrings.join("");
  if (!regExp.test(allStrings)) {
    var i = 0;
    while (i < matchStrings.length) {
      var string = matchStrings[i];
      assert(
        regExp.test(string),
        "`" + expression + "` should match " + string + " (" + printStringCodePoints(string) + ")"
      );
      i = i + 1;
    }
  }
  if (!nonMatchStrings) {
    return;
  }
  var allNonMatchStrings = nonMatchStrings.join("");
  if (regExp.test(allNonMatchStrings)) {
    var j = 0;
    while (j < nonMatchStrings.length) {
      var ns = nonMatchStrings[j];
      assert(
        !regExp.test(ns),
        "`" + expression + "` should not match " + ns + " (" + printStringCodePoints(ns) + ")"
      );
      j = j + 1;
    }
  }
}

// Same logic for extended character classes (`v` flag set operations).
var testExtendedCharacterClass = testPropertyOfStrings;

function matchValidator(expectedEntries, expectedIndex, expectedInput) {
  return function (match) {
    assert.compareArray(match, expectedEntries, "Match entries");
    assert.sameValue(match.index, expectedIndex, "Match index");
    assert.sameValue(match.input, expectedInput, "Match input");
  };
}
