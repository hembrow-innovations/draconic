(identifier) @variable
(type_identifier) @type
(predefined_type) @type.builtin
(property_identifier) @property
(shorthand_property_identifier) @property
(shorthand_property_identifier_pattern) @property
(private_property_identifier) @property

([
  (identifier)
  (type_identifier)
] @type.builtin
  (#any-of? @type.builtin
    "i8" "i16" "i32" "i64"
    "u8" "u16" "u32" "u64"
    "f32" "f64" "bool"))

((identifier) @keyword
  (#eq? @keyword "extern"))

(class_declaration
  name: (type_identifier) @type)

(type_alias_declaration
  name: (type_identifier) @type)

(interface_declaration
  name: (type_identifier) @type)

(new_expression
  constructor: (identifier) @constructor)

(call_expression
  function: (identifier) @function)

(call_expression
  function: (member_expression
    property: [
      (property_identifier)
      (private_property_identifier)
    ] @function))

(function_declaration
  name: (identifier) @function)

(function_expression
  name: (identifier) @function)

(method_definition
  name: [
    (property_identifier)
    (private_property_identifier)
  ] @function)

(method_definition
  name: (property_identifier) @constructor
  (#eq? @constructor "constructor"))

(variable_declarator
  name: (identifier) @function
  value: [
    (function_expression)
    (arrow_function)
  ])

(assignment_expression
  left: (identifier) @function
  right: [
    (function_expression)
    (arrow_function)
  ])

(pair
  key: [
    (property_identifier)
    (private_property_identifier)
  ] @function
  value: [
    (function_expression)
    (arrow_function)
  ])

(required_parameter (identifier) @variable.parameter)
(optional_parameter (identifier) @variable.parameter)
(arrow_function parameter: (identifier) @variable.parameter)
(catch_clause parameter: (identifier) @variable.parameter)

([
  (identifier)
  (shorthand_property_identifier)
  (shorthand_property_identifier_pattern)
] @constant
  (#match? @constant "^_*[A-Z_][A-Z\\d_]*$"))

(this) @variable.special
(super) @variable.special

[
  (null)
  (undefined)
] @constant.builtin

[
  (true)
  (false)
] @boolean

(comment) @comment
(hash_bang_line) @comment

[
  (string)
  (template_string)
  (template_literal_type)
] @string

(escape_sequence) @string.escape
(regex) @string.regex
(number) @number

[
  ";"
  "?."
  "."
  ","
  ":"
  "?"
] @punctuation.delimiter

[
  "-"
  "--"
  "-="
  "+"
  "++"
  "+="
  "*"
  "*="
  "**"
  "**="
  "/"
  "/="
  "%"
  "%="
  "<"
  "<="
  "<<"
  "<<="
  "="
  "=="
  "==="
  "!"
  "!="
  "!=="
  "=>"
  ">"
  ">="
  ">>"
  ">>="
  ">>>"
  ">>>="
  "~"
  "^"
  "&"
  "|"
  "^="
  "&="
  "|="
  "&&"
  "||"
  "??"
  "&&="
  "||="
  "??="
  "..."
] @operator

(ternary_expression
  [
    "?"
    ":"
  ] @operator)

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(template_substitution
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

(type_arguments
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

(type_parameters
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

[
  "abstract"
  "as"
  "async"
  "await"
  "break"
  "case"
  "catch"
  "class"
  "const"
  "continue"
  "debugger"
  "declare"
  "default"
  "delete"
  "do"
  "else"
  "export"
  "extends"
  "finally"
  "for"
  "from"
  "function"
  "get"
  "if"
  "implements"
  "import"
  "in"
  "instanceof"
  "interface"
  "let"
  "new"
  "of"
  "private"
  "protected"
  "public"
  "return"
  "set"
  "static"
  "switch"
  "throw"
  "try"
  "type"
  "typeof"
  "var"
  "void"
  "while"
  "with"
  "yield"
] @keyword
