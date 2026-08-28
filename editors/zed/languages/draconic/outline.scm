(function_declaration
  "async"? @context
  "function" @context
  name: (identifier) @name) @item

(class_declaration
  "class" @context
  name: (type_identifier) @name) @item

(lexical_declaration
  ["const" "let"] @context
  (variable_declarator
    name: (identifier) @name)) @item

(export_statement
  (function_declaration
    "async"? @context
    "function" @context
    name: (identifier) @name)) @item

(export_statement
  (class_declaration
    "class" @context
    name: (type_identifier) @name)) @item
