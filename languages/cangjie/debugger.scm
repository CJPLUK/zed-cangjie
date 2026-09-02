; debug-variable
; debug-scope
(varBindingPattern) @debug-variable
(scoped_identifier
    scope: (identifier) @debug-variable)
(atomicVariable
    .
    (identifier) @debug-variable)

(block) @debug-scope
