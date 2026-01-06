; Natsuzora fold queries

; All block constructs can be folded
(if_block) @fold
(unless_block) @fold
(each_block) @fold
(unsecure_block) @fold

; Else clause can be folded separately
(else_clause) @fold

; Comments can be folded
(comment) @fold
