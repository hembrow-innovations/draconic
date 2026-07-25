# Full ECMA-262 and Embed for eval

The destination is literally all of ECMA-262, including `eval`, `new Function`, and `with`. On the native target the Runtime includes Embed — enough of the Frontend/Compiler to compile those strings at run time. JS-backend-only eval and permanent omission were rejected as incomplete supersets.
