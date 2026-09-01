(
    (macroExpression
        "@"
        .
        (macroName) @_macroname
        (#eq? @_macroname "Test")) @run
    .
    (functionDefinition
        "func"
        .
        (funcName) @testname)
    (#set! tag cjpm-test)
)
