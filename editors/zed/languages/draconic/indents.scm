(if_statement consequence: (statement_block)? @end) @indent
(else_clause (statement_block)? @end) @indent
(for_statement body: (statement_block)? @end) @indent
(for_in_statement body: (statement_block)? @end) @indent
(while_statement body: (statement_block)? @end) @indent

(_ "[" "]" @end) @indent
(_ "{" "}" @end) @indent
(_ "(" ")" @end) @indent
